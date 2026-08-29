use base64::{Engine as _, engine::general_purpose::STANDARD};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set};
use uuid::Uuid;

use crate::{errors::AppError, services::s3_providers::S3ProviderClient};
use lib::entities::{bucket, bucket_grant, secret, storage};
use lib::entities::bucket::BucketStatus;
use lib::entities::secret::SecretScope;

pub async fn create(
    tx: &DatabaseTransaction,
    providers: &S3ProviderClient,
    organization_id: Uuid,
    region_id: Uuid,
    provider_id: Uuid,
) -> Result<Uuid, AppError> {
    let bucket_id = Uuid::new_v4();
    let secret_id = Uuid::new_v4();
    let first = Uuid::new_v4();
    let second = Uuid::new_v4();
    let mut key = [0_u8; 32];
    key[..16].copy_from_slice(first.as_bytes());
    key[16..].copy_from_slice(second.as_bytes());
    let ciphertext = lib::secrets::encrypt(
        &crate::state::get_app_state().secrets,
        &tenant_key(organization_id),
        STANDARD.encode(key).as_bytes(),
    )
    .await?;

    secret::ActiveModel {
        id: Set(secret_id),
        scope: Set(SecretScope::Tenant),
        organization_id: Set(Some(organization_id)),
        ciphertext: Set(ciphertext),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    bucket::ActiveModel {
        id: Set(bucket_id),
        region_id: Set(region_id),
        sse_secret_id: Set(secret_id),
        status: Set(BucketStatus::Active),
        ..Default::default()
    }
    .insert(tx)
    .await?;

    if let Err(error) = providers.create_bucket(provider_id, bucket_id).await {
        let _ = providers.delete_bucket(provider_id, bucket_id).await;
        return Err(error);
    }
    Ok(bucket_id)
}

pub async fn delete(tx: &DatabaseTransaction, bucket_id: Uuid) -> Result<(), AppError> {
    let foundation = bucket::Entity::find_by_id(bucket_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Bucket foundation not found".into()))?;
    bucket_grant::Entity::delete_many()
        .filter(bucket_grant::Column::BucketId.eq(bucket_id))
        .exec(tx)
        .await?;
    storage::Entity::delete_many()
        .filter(storage::Column::BucketId.eq(bucket_id))
        .exec(tx)
        .await?;
    bucket::Entity::delete_by_id(bucket_id).exec(tx).await?;
    secret::Entity::delete_by_id(foundation.sse_secret_id)
        .exec(tx)
        .await?;
    Ok(())
}

pub fn tenant_key(organization_id: Uuid) -> String {
    format!("tenant-{}", organization_id.simple())
}

#[cfg(test)]
mod tests {
    use super::tenant_key;
    use uuid::Uuid;

    #[test]
    fn derives_the_tenant_transit_key() {
        let id = Uuid::parse_str("67e55044-10b1-426f-9247-bb680e5fe0c8").unwrap();
        assert_eq!(tenant_key(id), "tenant-67e5504410b1426f9247bb680e5fe0c8");
    }
}

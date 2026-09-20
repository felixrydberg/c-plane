use sea_orm::{ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter};
use uuid::Uuid;

use crate::{
    entities::{bucket, bucket_grant, secret, storage},
    error::AppError,
    secrets::Client,
    services::s3_providers::S3ProviderClient,
};

pub async fn create(
    tx: &DatabaseTransaction,
    providers: &S3ProviderClient,
    secrets: &Client,
    organization_id: Uuid,
    region_id: Uuid,
    provider_id: Uuid,
) -> Result<Uuid, AppError> {
    let bucket_id = Uuid::new_v4();
    providers.create_bucket(provider_id, bucket_id).await?;
    if let Err(error) =
        crate::buckets::create_foundation(tx, secrets, organization_id, region_id, bucket_id).await
    {
        if let Err(delete_error) = providers.delete_bucket(provider_id, bucket_id).await {
            tracing::warn!(%provider_id, %bucket_id, %delete_error, "failed to compensate bucket after foundation error");
        }
        return Err(AppError::Internal(error.to_string()));
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

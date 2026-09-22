use sea_orm::{ColumnTrait, Condition, DatabaseTransaction, EntityTrait, QueryFilter, QuerySelect};
use uuid::Uuid;

use crate::{
    entities::{bucket, bucket_grant, secret, storage},
    error::AppError,
    secrets::Client,
    services::s3_providers::S3ProviderClient,
    tenant::{ScopedTenantTransaction, TenantDatabase},
};

pub async fn create(
    tenant_db: &TenantDatabase,
    providers: &S3ProviderClient,
    secrets: &Client,
    organization_id: Uuid,
    region_id: Uuid,
    provider_id: Uuid,
    bucket_id: Uuid,
) -> Result<ScopedTenantTransaction, AppError> {
    create_for_project(
        tenant_db,
        providers,
        secrets,
        organization_id,
        None,
        region_id,
        provider_id,
        bucket_id,
    )
    .await
}

pub async fn create_for_project(
    tenant_db: &TenantDatabase,
    providers: &S3ProviderClient,
    secrets: &Client,
    organization_id: Uuid,
    project_id: Option<Uuid>,
    region_id: Uuid,
    provider_id: Uuid,
    bucket_id: Uuid,
) -> Result<ScopedTenantTransaction, AppError> {
    providers.create_bucket(provider_id, bucket_id).await?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    crate::buckets::create_foundation_for_project(
        scoped.connection(),
        secrets,
        organization_id,
        project_id,
        region_id,
        bucket_id,
    )
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;
    Ok(scoped)
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

/// Removes all project-owned storage foundations in dependency order, including
/// pre-migration buckets discovered through their storage rows.
/// Deleting foundations preserves the database trigger that enqueues provider cleanup.
pub async fn delete_for_project(
    tx: &DatabaseTransaction,
    project_id: Uuid,
) -> Result<(), AppError> {
    let storage_bucket_ids = storage::Entity::find()
        .filter(storage::Column::ProjectId.eq(project_id))
        .select_only()
        .column(storage::Column::BucketId)
        .into_tuple::<Uuid>()
        .all(tx)
        .await?;
    let mut owned = Condition::any().add(bucket::Column::ProjectId.eq(project_id));
    if !storage_bucket_ids.is_empty() {
        owned = owned.add(bucket::Column::Id.is_in(storage_bucket_ids));
    }
    let foundations = bucket::Entity::find().filter(owned).all(tx).await?;
    let foundation_ids = foundations.iter().map(|row| row.id).collect::<Vec<_>>();
    if !foundation_ids.is_empty() {
        bucket_grant::Entity::delete_many()
            .filter(bucket_grant::Column::BucketId.is_in(foundation_ids.clone()))
            .exec(tx)
            .await?;
    }
    storage::Entity::delete_many()
        .filter(storage::Column::ProjectId.eq(project_id))
        .exec(tx)
        .await?;
    if !foundation_ids.is_empty() {
        bucket::Entity::delete_many()
            .filter(bucket::Column::Id.is_in(foundation_ids))
            .exec(tx)
            .await?;
    }
    let secret_ids = foundations
        .into_iter()
        .map(|foundation| foundation.sse_secret_id)
        .collect::<Vec<_>>();
    if !secret_ids.is_empty() {
        secret::Entity::delete_many()
            .filter(secret::Column::Id.is_in(secret_ids))
            .exec(tx)
            .await?;
    }
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

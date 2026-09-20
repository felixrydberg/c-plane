use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseTransaction,
    EntityTrait, QueryFilter, Set, Statement,
};
use uuid::Uuid;

use crate::entities::{
    bucket, bucket_grant, organization_region_backup_bucket,
    region::{self, RegionRoutingMode, RegionStatus},
    s3_provider,
};
use crate::error::AppError;
use crate::secrets::Client;
use crate::services::{buckets, s3_providers::S3ProviderClient};
use crate::tenant::TenantDatabase;

pub struct BackupBucket {
    pub mapping_id: Uuid,
    pub bucket_id: Uuid,
}

pub async fn ensure_backup_bucket(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    region_id: Uuid,
    providers: &S3ProviderClient,
    secrets: &Client,
) -> Result<BackupBucket, AppError> {
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    lock_region(tx, organization_id, region_id).await?;

    let region = region::Entity::find_by_id(region_id)
        .filter(region::Column::Status.eq(RegionStatus::Active))
        .filter(region::Column::RoutingMode.ne(RegionRoutingMode::Disabled))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Active region not found".into()))?;
    let provider_id = region
        .s3_provider_id
        .ok_or_else(|| AppError::Conflict("Region has no S3 provider".into()))?;
    let provider_is_active = s3_provider::Entity::find_by_id(provider_id)
        .filter(s3_provider::Column::IsActive.eq(true))
        .one(tx)
        .await?
        .is_some();
    if !provider_is_active {
        return Err(AppError::Conflict("S3 provider is not active".into()));
    }

    let mapping = organization_region_backup_bucket::Entity::find()
        .filter(organization_region_backup_bucket::Column::OrganizationId.eq(organization_id))
        .filter(organization_region_backup_bucket::Column::RegionId.eq(region_id))
        .one(tx)
        .await?;

    if let Some(mapping) = mapping {
        let foundation = bucket::Entity::find_by_id(mapping.bucket_id)
            .one(tx)
            .await?
            .ok_or_else(|| AppError::Conflict("Backup bucket not found".into()))?;
        if foundation.status != bucket::BucketStatus::Active {
            return Err(AppError::Conflict("Backup bucket is not active".into()));
        }
        let backup_bucket = BackupBucket {
            mapping_id: mapping.id,
            bucket_id: mapping.bucket_id,
        };
        scoped.commit().await?;
        return Ok(backup_bucket);
    }

    let bucket_id = buckets::create(
        tx,
        providers,
        secrets,
        organization_id,
        region_id,
        provider_id,
    )
    .await?;
    let mapping = match (organization_region_backup_bucket::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(organization_id),
        region_id: Set(region_id),
        bucket_id: Set(bucket_id),
    })
    .insert(tx)
    .await
    {
        Ok(mapping) => mapping,
        Err(error) => {
            compensate_created_bucket(providers, provider_id, bucket_id).await;
            return Err(error.into());
        }
    };

    let backup_bucket = BackupBucket {
        mapping_id: mapping.id,
        bucket_id,
    };
    if let Err(error) = scoped.commit().await {
        compensate_after_commit_error(
            tenant_db,
            organization_id,
            region_id,
            provider_id,
            bucket_id,
            providers,
        )
        .await;
        return Err(error);
    }
    Ok(backup_bucket)
}

async fn compensate_after_commit_error(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    region_id: Uuid,
    provider_id: Uuid,
    bucket_id: Uuid,
    providers: &S3ProviderClient,
) {
    let verification = async {
        let scoped = tenant_db.begin_scoped_transaction().await?;
        let tx = scoped.connection();
        lock_region(tx, organization_id, region_id).await?;
        let bucket = bucket::Entity::find_by_id(bucket_id).one(tx).await?;
        scoped.commit().await?;
        Ok::<_, AppError>(bucket.is_some())
    }
    .await;

    match verification {
        Ok(true) => {}
        Ok(false) => compensate_created_bucket(providers, provider_id, bucket_id).await,
        Err(error) => {
            tracing::warn!(%organization_id, %region_id, %bucket_id, %error, "failed to verify bucket after commit error; preserving bucket");
        }
    }
}

async fn compensate_created_bucket(
    providers: &S3ProviderClient,
    provider_id: Uuid,
    bucket_id: Uuid,
) {
    if let Err(error) = providers.delete_bucket(provider_id, bucket_id).await {
        tracing::warn!(%provider_id, %bucket_id, %error, "failed to compensate bucket");
    }
}

pub async fn provision_access(
    tx: &DatabaseTransaction,
    secrets: &Client,
    organization_id: Uuid,
    bucket_id: Uuid,
    credential_name_prefix: &str,
    object_prefix: &str,
) -> Result<Uuid, AppError> {
    let credential = crate::buckets::credentials::create(
        secrets,
        Some(organization_id),
        format!("{}{}", credential_name_prefix, Uuid::new_v4().simple()).to_uppercase(),
        object_prefix.to_owned(),
        &serde_json::json!({"secret_access_key": format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())}),
    )
    .await
    .map_err(|error| AppError::Internal(error.to_string()))?;
    crate::buckets::credentials::insert(tx, &credential)
        .await
        .map_err(|error| AppError::Internal(error.to_string()))?;
    (bucket_grant::ActiveModel {
        id: Set(Uuid::new_v4()),
        credential_id: Set(credential.id),
        bucket_id: Set(bucket_id),
        organization_id: Set(Some(organization_id)),
        prefix: Set(object_prefix.to_owned()),
        can_read: Set(true),
        can_write: Set(true),
        ..Default::default()
    })
    .insert(tx)
    .await?;
    Ok(credential.id)
}

pub(crate) async fn lock_region(
    tx: &impl ConnectionTrait,
    organization_id: Uuid,
    region_id: Uuid,
) -> Result<(), AppError> {
    tx.execute(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        vec![format!("backup:{organization_id}:{region_id}").into()],
    ))
    .await?;
    Ok(())
}

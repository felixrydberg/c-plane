use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseTransaction,
    EntityTrait, QueryFilter, Set, Statement,
};
use uuid::Uuid;

use crate::entities::{
    bucket, bucket_grant, organization_region_backup_bucket, postgres_database_branch,
    region::{self, RegionRoutingMode, RegionStatus},
    s3_provider,
};
use crate::error::AppError;
use crate::operation::{Operation, foundation_bucket_delete::FoundationBucketDelete};
use crate::retry::{Retry, RetryError};
use crate::secrets::Client;
use crate::services::{buckets, s3_providers::S3ProviderClient};
use crate::tenant::TenantDatabase;
use std::time::Duration;

#[derive(Clone, Copy)]
pub struct BackupBucket {
    pub mapping_id: Uuid,
    pub bucket_id: Uuid,
    pub provider_id: Uuid,
    pub created: bool,
}

pub async fn ensure_backup_bucket(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    region_id: Uuid,
    providers: &S3ProviderClient,
    secrets: &Client,
) -> Result<BackupBucket, AppError> {
    let bucket_id = Uuid::new_v4();
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
            provider_id,
            created: false,
        };
        scoped.commit().await?;
        return Ok(backup_bucket);
    }
    scoped.commit().await?;
    let scoped = Retry::new(3, Duration::from_secs(10), Duration::from_millis(250))
        .run(
            || {
                buckets::create(
                    tenant_db,
                    providers,
                    secrets,
                    organization_id,
                    region_id,
                    provider_id,
                    bucket_id,
                )
            },
            |_| true,
        )
        .await
        .map_err(|error| match error {
            RetryError::Failed(error) => error,
            RetryError::TimedOut => {
                AppError::ServiceUnavailable("Bucket creation timed out".into())
            }
        })?;
    let tx = scoped.connection();
    lock_region(tx, organization_id, region_id).await?;
    if let Some(mapping) = organization_region_backup_bucket::Entity::find()
        .filter(organization_region_backup_bucket::Column::OrganizationId.eq(organization_id))
        .filter(organization_region_backup_bucket::Column::RegionId.eq(region_id))
        .one(tx)
        .await?
    {
        let foundation = bucket::Entity::find_by_id(mapping.bucket_id)
            .one(tx)
            .await?
            .ok_or_else(|| AppError::Conflict("Backup bucket not found".into()))?;
        let backup_bucket = BackupBucket {
            mapping_id: mapping.id,
            bucket_id: mapping.bucket_id,
            provider_id,
            created: false,
        };
        drop(scoped);
        compensate_created_bucket(providers, provider_id, bucket_id).await;
        if foundation.status != bucket::BucketStatus::Active {
            return Err(AppError::Conflict("Backup bucket is not active".into()));
        }
        return Ok(backup_bucket);
    }
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
        provider_id,
        created: true,
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

pub async fn rollback_created_backup_bucket(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    region_id: Uuid,
    backup_bucket: BackupBucket,
) -> Result<(), AppError> {
    if !backup_bucket.created {
        return Ok(());
    }

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    lock_region(tx, organization_id, region_id).await?;

    let has_branches = postgres_database_branch::Entity::find()
        .filter(
            postgres_database_branch::Column::OrganizationRegionBackupBucketId
                .eq(backup_bucket.mapping_id),
        )
        .filter(postgres_database_branch::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .is_some();
    if has_branches {
        scoped.commit().await?;
        return Ok(());
    }

    let mapping = organization_region_backup_bucket::Entity::find_by_id(backup_bucket.mapping_id)
        .filter(organization_region_backup_bucket::Column::OrganizationId.eq(organization_id))
        .filter(organization_region_backup_bucket::Column::RegionId.eq(region_id))
        .one(tx)
        .await?;
    if let Some(mapping) = mapping.filter(|mapping| mapping.bucket_id == backup_bucket.bucket_id) {
        organization_region_backup_bucket::Entity::delete_by_id(mapping.id)
            .exec(tx)
            .await?;
        crate::buckets::delete_foundation(tx, mapping.bucket_id).await?;
        Operation::<FoundationBucketDelete>::new(
            tx,
            organization_id,
            mapping.bucket_id.to_string(),
            FoundationBucketDelete {
                bucket_id: mapping.bucket_id,
                provider_id: backup_bucket.provider_id,
            },
        )
        .await?;
    }

    scoped.commit().await?;
    Ok(())
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

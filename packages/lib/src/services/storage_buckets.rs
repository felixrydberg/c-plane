use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set,
    TryInsertResult,
};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    entities::{
        project,
        region::{self, RegionRoutingMode, RegionStatus},
        storage, storage_bucket_name_reservation,
    },
    error::AppError,
    retry::{Retry, RetryError},
    secrets::Client,
    services::{buckets, events, s3_providers::S3ProviderClient},
    tenant::TenantDatabase,
};

pub struct CreateStorageBucketInput {
    pub project_id: Uuid,
    pub name: String,
    pub region_id: Uuid,
}

pub struct CreatedStorageBucket {
    pub bucket: storage::Model,
    pub region: region::Model,
}

pub async fn create(
    tenant_db: &TenantDatabase,
    providers: &S3ProviderClient,
    secrets: &Client,
    organization_id: Uuid,
    actor_id: Uuid,
    input: CreateStorageBucketInput,
) -> Result<CreatedStorageBucket, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let name = input.name.trim().to_ascii_lowercase();
    if !valid_bucket_name(&name) {
        return Err(AppError::BadRequest("Invalid bucket name".into()));
    }

    let foundation_bucket_id = Uuid::new_v4();
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, input.project_id, organization_id).await?;
    let region = region::Entity::find_by_id(input.region_id)
        .filter(region::Column::Status.eq(RegionStatus::Active))
        .filter(region::Column::RoutingMode.ne(RegionRoutingMode::Disabled))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Active region not found".into()))?;
    let provider_id = region
        .s3_provider_id
        .ok_or_else(|| AppError::Conflict("Region has no S3 provider".into()))?;
    reserve_name(
        tx,
        organization_id,
        input.project_id,
        &name,
        foundation_bucket_id,
        region.id,
        provider_id,
    )
    .await?;
    if storage::Entity::find()
        .filter(storage::Column::ProjectId.eq(input.project_id))
        .filter(storage::Column::Name.eq(&name))
        .one(tx)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "Bucket name is already reserved in this project".into(),
        ));
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
                    region.id,
                    provider_id,
                    foundation_bucket_id,
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
    let created = match (storage::ActiveModel {
        id: Set(Uuid::new_v4()),
        project_id: Set(input.project_id),
        organization_id: Set(organization_id),
        bucket_id: Set(foundation_bucket_id),
        name: Set(name.clone()),
    })
    .insert(tx)
    .await
    {
        Ok(created) => created,
        Err(error) => {
            drop(scoped);
            let _ = providers
                .delete_bucket(provider_id, foundation_bucket_id)
                .await;
            return Err(error.into());
        }
    };
    if let Err(error) = events::record(
        tx,
        organization_id,
        input.project_id,
        "bucket:created",
        json!({ "summary": format!("Created bucket '{name}'"), "target_id": created.id }),
        actor_id,
    )
    .await
    {
        drop(scoped);
        let _ = providers
            .delete_bucket(provider_id, foundation_bucket_id)
            .await;
        return Err(error);
    }
    if let Err(error) = release_name(
        tx,
        organization_id,
        input.project_id,
        &name,
        foundation_bucket_id,
    )
    .await
    {
        drop(scoped);
        let _ = providers
            .delete_bucket(provider_id, foundation_bucket_id)
            .await;
        return Err(error);
    }
    scoped.commit().await?;
    Ok(CreatedStorageBucket {
        bucket: created,
        region,
    })
}

async fn reserve_name(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
    name: &str,
    bucket_id: Uuid,
    region_id: Uuid,
    provider_id: Uuid,
) -> Result<(), AppError> {
    let result = storage_bucket_name_reservation::Entity::insert(
        storage_bucket_name_reservation::ActiveModel {
            organization_id: Set(organization_id),
            project_id: Set(project_id),
            name: Set(name.to_owned()),
            bucket_id: Set(bucket_id),
            region_id: Set(region_id),
            provider_id: Set(provider_id),
            created_at: Set(chrono::Utc::now().into()),
        },
    )
    .on_conflict_do_nothing()
    .exec(tx)
    .await?;
    match result {
        TryInsertResult::Inserted(_) => Ok(()),
        TryInsertResult::Conflicted | TryInsertResult::Empty => Err(AppError::Conflict(
            "Bucket name is already being created or requires explicit recovery".into(),
        )),
    }
}

async fn release_name(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
    name: &str,
    bucket_id: Uuid,
) -> Result<(), AppError> {
    let result = storage_bucket_name_reservation::Entity::delete_many()
        .filter(storage_bucket_name_reservation::Column::OrganizationId.eq(organization_id))
        .filter(storage_bucket_name_reservation::Column::ProjectId.eq(project_id))
        .filter(storage_bucket_name_reservation::Column::Name.eq(name))
        .filter(storage_bucket_name_reservation::Column::BucketId.eq(bucket_id))
        .exec(tx)
        .await?;
    if result.rows_affected != 1 {
        return Err(AppError::Conflict(
            "Creation reservation is no longer owned by this attempt".into(),
        ));
    }
    Ok(())
}

fn verify_org_access(tenant_db: &TenantDatabase, organization_id: Uuid) -> Result<(), AppError> {
    if tenant_db
        .context
        .allowed_organizations
        .contains(&organization_id)
    {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "You do not have access to this organization".into(),
        ))
    }
}

async fn verify_project_in_org(
    tx: &impl sea_orm::ConnectionTrait,
    project_id: Uuid,
    organization_id: Uuid,
) -> Result<(), AppError> {
    if project::Entity::find()
        .filter(project::Column::Id.eq(project_id))
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .is_some()
    {
        Ok(())
    } else {
        Err(AppError::NotFound(
            "Project not found in this organization".into(),
        ))
    }
}

fn valid_bucket_name(name: &str) -> bool {
    (3..=63).contains(&name.len())
        && !name.starts_with(['.', '-'])
        && !name.ends_with(['.', '-'])
        && name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::valid_bucket_name;

    #[test]
    fn validates_s3_bucket_name_basics() {
        assert!(valid_bucket_name("project-assets"));
        assert!(!valid_bucket_name("UPPERCASE"));
        assert!(!valid_bucket_name("ab"));
        assert!(!valid_bucket_name("-assets"));
    }
}

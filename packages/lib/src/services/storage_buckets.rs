use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, QueryOrder, Set,
    TryInsertResult,
};
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    entities::{
        bucket, bucket_grant, project,
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

pub struct ListedStorageBucket {
    pub bucket: storage::Model,
    pub region: region::Model,
}

pub struct DeletedStorageBucket {
    pub bucket: storage::Model,
}

pub async fn delete_project_in_transaction(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
    actor_id: Uuid,
) -> Result<(), AppError> {
    let rows = storage::Entity::find()
        .filter(storage::Column::OrganizationId.eq(organization_id))
        .filter(storage::Column::ProjectId.eq(project_id))
        .all(tx)
        .await?;
    buckets::delete_for_project(tx, project_id).await?;
    if !rows.is_empty() {
        let events = rows.into_iter().map(|row| crate::entities::event::ActiveModel {
            id: Set(Uuid::new_v4()),
            organization_id: Set(organization_id),
            project_id: Set(Some(project_id)),
            event_type: Set("bucket:deleted".into()),
            payload: Set(json!({"summary": format!("Deleted bucket '{}'", row.name), "target_id": row.id})),
            system: Set(false),
            actor_id: Set(Some(actor_id)),
            created_at: Set(chrono::Utc::now().fixed_offset()),
        }).collect::<Vec<_>>();
        crate::entities::event::Entity::insert_many(events)
            .exec(tx)
            .await?;
    }
    Ok(())
}

pub async fn delete(
    tenant_db: &TenantDatabase,
    providers: &S3ProviderClient,
    organization_id: Uuid,
    bucket_id: Uuid,
    actor_id: Uuid,
) -> Result<DeletedStorageBucket, AppError> {
    verify_org_owner(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let (bucket_row, foundation, region) = storage::Entity::find_by_id(bucket_id)
        .filter(storage::Column::OrganizationId.eq(organization_id))
        .find_also_related(bucket::Entity)
        .and_also_related(region::Entity)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Bucket not found".into()))?;
    verify_project_in_org(tx, bucket_row.project_id, organization_id).await?;
    let _foundation =
        foundation.ok_or_else(|| AppError::NotFound("Bucket foundation not found".into()))?;
    let region = region.ok_or_else(|| AppError::NotFound("Bucket region not found".into()))?;
    let provider_id = region
        .s3_provider_id
        .ok_or_else(|| AppError::Conflict("Region has no S3 provider".into()))?;
    let access_keys = bucket_grant::Entity::find()
        .filter(bucket_grant::Column::BucketId.eq(bucket_row.bucket_id))
        .find_also_related(crate::entities::credential::Entity)
        .all(tx)
        .await?
        .into_iter()
        .filter_map(|(_, credential)| credential.map(|credential| credential.access_key_id))
        .collect::<Vec<_>>();
    bucket::ActiveModel {
        id: Set(bucket_row.bucket_id),
        status: Set(bucket::BucketStatus::Deleting),
        ..Default::default()
    }
    .update(tx)
    .await?;
    scoped.commit().await?;
    providers
        .invalidate_access_token_caches(&access_keys)
        .await?;
    if !providers
        .bucket_is_empty(provider_id, bucket_row.bucket_id)
        .await?
    {
        let scoped = tenant_db.begin_scoped_transaction().await?;
        bucket::ActiveModel {
            id: Set(bucket_row.bucket_id),
            status: Set(bucket::BucketStatus::Active),
            ..Default::default()
        }
        .update(scoped.connection())
        .await?;
        scoped.commit().await?;
        let _ = providers.invalidate_access_token_caches(&access_keys).await;
        return Err(AppError::Conflict(
            "Bucket must be empty before it can be deleted".into(),
        ));
    }
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    crate::services::buckets::delete(tx, bucket_row.bucket_id).await?;
    events::record(
        tx,
        organization_id,
        bucket_row.project_id,
        "bucket:deleted",
        json!({ "summary": format!("Deleted bucket '{}'", bucket_row.name), "target_id": bucket_row.id }),
        actor_id,
    ).await?;
    scoped.commit().await?;
    Ok(DeletedStorageBucket { bucket: bucket_row })
}

pub async fn list(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<Vec<ListedStorageBucket>, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;
    let rows = storage::Entity::find()
        .filter(storage::Column::ProjectId.eq(project_id))
        .order_by_asc(storage::Column::Name)
        .find_also_related(bucket::Entity)
        .and_also_related(region::Entity)
        .all(tx)
        .await?;
    let result = rows
        .into_iter()
        .map(|(bucket, foundation, region)| {
            foundation.ok_or_else(|| AppError::NotFound("Bucket foundation not found".into()))?;
            Ok(ListedStorageBucket {
                bucket,
                region: region
                    .ok_or_else(|| AppError::NotFound("Bucket region not found".into()))?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    scoped.commit().await?;
    Ok(result)
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
                buckets::create_for_project(
                    tenant_db,
                    providers,
                    secrets,
                    organization_id,
                    Some(input.project_id),
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

fn verify_org_owner(tenant_db: &TenantDatabase, organization_id: Uuid) -> Result<(), AppError> {
    verify_org_access(tenant_db, organization_id)?;
    if tenant_db.context.api_key_organization_id == Some(organization_id)
        || tenant_db
            .context
            .organization_roles
            .get(&organization_id)
            .map(String::as_str)
            == Some("owner")
    {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "Organization owner role required".into(),
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

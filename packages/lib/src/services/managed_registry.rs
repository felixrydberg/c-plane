use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseTransaction,
    EntityTrait, QueryFilter, Set, Statement, TryInsertResult,
};
use serde::Serialize;
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    entities::{
        bucket, bucket_grant, credential, event,
        managed_registry::{self, ManagedRegistryStatus},
        managed_registry_activation_reservation,
        region::{self, RegionRoutingMode, RegionStatus},
        secret::{self, SecretScope},
    },
    error::AppError,
    retry::{Retry, RetryError},
    secrets::{self, Client},
    services::{buckets, s3_providers::S3ProviderClient},
    tenant::TenantDatabase,
};

const ACCESS_KEY_PREFIX: &str = "CP";

#[derive(Serialize)]
struct S3SecretKey {
    secret_access_key: String,
}

pub struct ActivatedManagedRegistry {
    pub registry: managed_registry::Model,
    pub region_id: Uuid,
    pub created: bool,
}

pub async fn activate(
    tenant_db: &TenantDatabase,
    providers: &S3ProviderClient,
    secrets_client: &Client,
    organization_id: Uuid,
    actor_id: Uuid,
    region_id: Uuid,
) -> Result<ActivatedManagedRegistry, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let foundation_bucket_id = Uuid::new_v4();
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    lock_organization(tx, organization_id).await?;

    if let Some((existing, foundation)) = managed_registry::Entity::find_by_id(organization_id)
        .find_also_related(bucket::Entity)
        .one(tx)
        .await?
    {
        let foundation = foundation
            .ok_or_else(|| AppError::NotFound("Managed Registry bucket not found".into()))?;
        scoped.commit().await?;
        return Ok(ActivatedManagedRegistry {
            registry: existing,
            region_id: foundation.region_id,
            created: false,
        });
    }

    let region = region::Entity::find_by_id(region_id)
        .filter(region::Column::Status.eq(RegionStatus::Active))
        .filter(region::Column::RoutingMode.ne(RegionRoutingMode::Disabled))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Active region not found".into()))?;
    let provider_id = region
        .s3_provider_id
        .ok_or_else(|| AppError::Conflict("Region has no S3 provider".into()))?;
    reserve_activation(
        tx,
        organization_id,
        foundation_bucket_id,
        region.id,
        provider_id,
    )
    .await?;
    scoped.commit().await?;

    let scoped = Retry::new(3, Duration::from_secs(10), Duration::from_millis(250))
        .run(
            || {
                buckets::create(
                    tenant_db,
                    providers,
                    secrets_client,
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
    let registry =
        match provision_metadata(tx, secrets_client, organization_id, foundation_bucket_id).await {
            Ok(registry) => registry,
            Err(error) => {
                drop(scoped);
                let _ = providers
                    .delete_bucket(provider_id, foundation_bucket_id)
                    .await;
                return Err(error);
            }
        };
    if let Err(error) = record_event(tx, organization_id, actor_id).await {
        drop(scoped);
        let _ = providers
            .delete_bucket(provider_id, foundation_bucket_id)
            .await;
        return Err(error);
    }
    if let Err(error) = release_activation(tx, organization_id, foundation_bucket_id).await {
        drop(scoped);
        let _ = providers
            .delete_bucket(provider_id, foundation_bucket_id)
            .await;
        return Err(error);
    }
    scoped.commit().await?;
    Ok(ActivatedManagedRegistry {
        registry,
        region_id: region.id,
        created: true,
    })
}

async fn reserve_activation(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    bucket_id: Uuid,
    region_id: Uuid,
    provider_id: Uuid,
) -> Result<(), AppError> {
    let result = managed_registry_activation_reservation::Entity::insert(
        managed_registry_activation_reservation::ActiveModel {
            organization_id: Set(organization_id),
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
            "Managed Registry activation is already in progress or requires explicit recovery"
                .into(),
        )),
    }
}

async fn release_activation(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    bucket_id: Uuid,
) -> Result<(), AppError> {
    let result = managed_registry_activation_reservation::Entity::delete_many()
        .filter(managed_registry_activation_reservation::Column::OrganizationId.eq(organization_id))
        .filter(managed_registry_activation_reservation::Column::BucketId.eq(bucket_id))
        .exec(tx)
        .await?;
    if result.rows_affected != 1 {
        return Err(AppError::Conflict(
            "Managed Registry activation reservation is no longer owned by this attempt".into(),
        ));
    }
    Ok(())
}

async fn provision_metadata(
    tx: &DatabaseTransaction,
    secrets_client: &Client,
    organization_id: Uuid,
    foundation_bucket_id: Uuid,
) -> Result<managed_registry::Model, AppError> {
    let credential_id = Uuid::new_v4();
    let secret_id = Uuid::new_v4();
    let access_key_id = format!("{ACCESS_KEY_PREFIX}{}", Uuid::new_v4().simple()).to_uppercase();
    let secret_access_key = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let plaintext = serde_json::to_vec(&S3SecretKey { secret_access_key })
        .map_err(|error| AppError::Internal(error.to_string()))?;
    let ciphertext = secrets::encrypt(
        secrets_client,
        &buckets::tenant_key(organization_id),
        &plaintext,
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
    credential::ActiveModel {
        id: Set(credential_id),
        organization_id: Set(Some(organization_id)),
        access_key_id: Set(access_key_id),
        prefix: Set(String::new()),
        secret_id: Set(secret_id),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    bucket_grant::ActiveModel {
        id: Set(Uuid::new_v4()),
        credential_id: Set(credential_id),
        bucket_id: Set(foundation_bucket_id),
        organization_id: Set(Some(organization_id)),
        prefix: Set(String::new()),
        can_read: Set(true),
        can_write: Set(true),
        ..Default::default()
    }
    .insert(tx)
    .await?;
    Ok(managed_registry::ActiveModel {
        organization_id: Set(organization_id),
        bucket_id: Set(foundation_bucket_id),
        credential_id: Set(credential_id),
        status: Set(ManagedRegistryStatus::Active),
        storage_revision: Set(Uuid::new_v4()),
        ..Default::default()
    }
    .insert(tx)
    .await?)
}

async fn record_event(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    actor_id: Uuid,
) -> Result<(), AppError> {
    event::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(organization_id),
        event_type: Set("managed-registry:activated".into()),
        payload: Set(json!({
            "summary": "Activated managed Registry",
            "target_id": organization_id,
        })),
        system: Set(false),
        project_id: Set(None),
        actor_id: Set(Some(actor_id)),
        created_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;
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

async fn lock_organization(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
) -> Result<(), AppError> {
    tx.query_one(Statement::from_sql_and_values(
        DatabaseBackend::Postgres,
        "SELECT id FROM organization WHERE id=$1 FOR UPDATE",
        vec![organization_id.into()],
    ))
    .await?
    .ok_or_else(|| AppError::NotFound("Organization not found".into()))?;
    Ok(())
}

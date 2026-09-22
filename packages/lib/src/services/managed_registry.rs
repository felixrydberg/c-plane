use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseBackend, DatabaseTransaction,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set, Statement, TryInsertResult,
};
use serde::Serialize;
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

use crate::{
    entities::{
        bucket, bucket_grant, credential, event,
        managed_registry::{self, ManagedRegistryStatus},
        managed_registry_activation_reservation, managed_registry_gc_run, project,
        region::{self, RegionRoutingMode, RegionStatus},
        registry_repository,
        secret::{self, SecretScope},
    },
    error::AppError,
    operation::{Operation, registry_gc::RegistryGc},
    retry::{Retry, RetryError},
    secrets::{self, Client},
    services::{buckets, registry::normalize_project_name, s3_providers::S3ProviderClient},
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

pub struct ManagedRegistryView {
    pub registry: managed_registry::Model,
    pub region_id: Uuid,
}

pub struct GarbageCollectionView {
    pub registry: managed_registry::Model,
    pub active_job: Option<ActiveGcJob>,
    pub runs: Vec<managed_registry_gc_run::Model>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}

pub async fn garbage_collection(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    page: u64,
    per_page: u64,
) -> Result<GarbageCollectionView, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let registry = managed_registry::Entity::find_by_id(organization_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Managed Registry is not activated".into()))?;
    let active_job = active_gc_job(tx, &registry).await?;
    let query = managed_registry_gc_run::Entity::find()
        .filter(managed_registry_gc_run::Column::OrganizationId.eq(organization_id));
    let total = query.clone().count(tx).await?;
    let runs = query
        .order_by_desc(managed_registry_gc_run::Column::StartedAt)
        .paginate(tx, per_page)
        .fetch_page(page.saturating_sub(1))
        .await?;
    scoped.commit().await?;
    Ok(GarbageCollectionView {
        registry,
        active_job,
        runs,
        total,
        page,
        per_page,
    })
}

pub async fn queue_garbage_collection(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
) -> Result<GarbageCollectionView, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    lock_organization(tx, organization_id).await?;
    let registry = managed_registry::Entity::find_by_id(organization_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Managed Registry is not activated".into()))?;
    if let Some(job) = active_gc_job(tx, &registry).await? {
        if job.status == "running" {
            return Err(AppError::Conflict(
                "Registry garbage collection is already running".into(),
            ));
        }
        tx.execute(Statement::from_sql_and_values(DatabaseBackend::Postgres, "UPDATE worker_queue SET available_at=NOW(), payload=jsonb_build_object('trigger', 'manual'), updated_at=NOW() WHERE id=$1 AND organization_id=$2 AND queue_name=$3 AND job_type=$4 AND status='queued'", vec![job.id.into(), organization_id.into(), Operation::<RegistryGc>::QUEUE.into(), Operation::<RegistryGc>::NAME.into()])).await?;
    } else {
        Operation::<RegistryGc>::new(tx, organization_id, "manual").await?;
    }
    event::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(organization_id),
        event_type: Set("registry-garbage-collection:queued".into()),
        payload: Set(json!({"summary":"Queued managed Registry garbage collection"})),
        system: Set(false),
        project_id: Set(None),
        actor_id: Set(Some(actor_id)),
        created_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;
    let registry = managed_registry::Entity::find_by_id(organization_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Managed Registry is not activated".into()))?;
    let active_job = active_gc_job(tx, &registry).await?;
    let query = managed_registry_gc_run::Entity::find()
        .filter(managed_registry_gc_run::Column::OrganizationId.eq(organization_id));
    let total = query.clone().count(tx).await?;
    let runs = query
        .order_by_desc(managed_registry_gc_run::Column::StartedAt)
        .paginate(tx, 10)
        .fetch_page(0)
        .await?;
    scoped.commit().await?;
    Ok(GarbageCollectionView {
        registry,
        active_job,
        runs,
        total,
        page: 1,
        per_page: 10,
    })
}

async fn active_gc_job(
    tx: &DatabaseTransaction,
    registry: &managed_registry::Model,
) -> Result<Option<ActiveGcJob>, AppError> {
    let Some(job_id) = registry.gc_active_job_id else {
        return Ok(None);
    };
    let row = tx.query_one(Statement::from_sql_and_values(DatabaseBackend::Postgres, "SELECT id, status, COALESCE(payload->>'trigger', 'manual') AS trigger, available_at FROM worker_queue WHERE id=$1 AND organization_id=$2 AND queue_name=$3 AND job_type=$4 AND status IN ('queued', 'running') LIMIT 1", vec![job_id.into(), registry.organization_id.into(), Operation::<RegistryGc>::QUEUE.into(), Operation::<RegistryGc>::NAME.into()])).await?;
    row.map(|row| {
        Ok(ActiveGcJob {
            id: row.try_get("", "id")?,
            status: row.try_get("", "status")?,
            trigger: row.try_get("", "trigger")?,
            available_at: row.try_get("", "available_at")?,
        })
    })
    .transpose()
}

pub struct ActiveGcJob {
    pub id: Uuid,
    pub status: String,
    pub trigger: String,
    pub available_at: chrono::DateTime<chrono::FixedOffset>,
}

pub struct ResolvedRegistry {
    pub organization_id: Uuid,
    pub organization_slug: String,
    pub storage_revision: Uuid,
    pub status: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    pub repository_name: Option<String>,
    pub repository_id: Option<Uuid>,
}

pub async fn resolve(
    database: &sea_orm::DatabaseConnection,
    secrets_client: &Client,
    organization_id: Uuid,
    repository_name: Option<String>,
    repository_id: Option<Uuid>,
) -> Result<ResolvedRegistry, AppError> {
    let (registry, credential, credential_secret) =
        managed_registry::Entity::find_by_id(organization_id)
            .find_also_related(credential::Entity)
            .and_also_related(secret::Entity)
            .one(database)
            .await?
            .ok_or_else(|| AppError::NotFound("Managed Registry is not activated".into()))?;
    let credential = credential
        .filter(|c| c.organization_id == Some(organization_id) && c.revoked_at.is_none())
        .ok_or_else(|| AppError::NotFound("Managed Registry credential not found".into()))?;
    let credential_secret = credential_secret
        .filter(|s| s.scope == SecretScope::Tenant && s.organization_id == Some(organization_id))
        .ok_or_else(|| AppError::NotFound("Managed Registry secret not found".into()))?;
    let grant = bucket_grant::Entity::find()
        .filter(bucket_grant::Column::CredentialId.eq(credential.id))
        .filter(bucket_grant::Column::OrganizationId.eq(organization_id))
        .filter(bucket_grant::Column::BucketId.eq(registry.bucket_id))
        .one(database)
        .await?
        .ok_or_else(|| AppError::NotFound("Managed Registry grant not found".into()))?;
    if !grant.can_read || !grant.can_write {
        return Err(AppError::Conflict(
            "Managed Registry grant must allow reads and writes".into(),
        ));
    }
    let plaintext = secrets::decrypt(
        secrets_client,
        &buckets::tenant_key(organization_id),
        &credential_secret.ciphertext,
    )
    .await?;
    #[derive(serde::Deserialize)]
    struct S3Secret {
        secret_access_key: String,
    }
    let s3_secret: S3Secret =
        serde_json::from_slice(&plaintext).map_err(|e| AppError::Internal(e.to_string()))?;
    let organization = database
        .query_one(Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT slug FROM organization WHERE id=$1 LIMIT 1",
            vec![organization_id.into()],
        ))
        .await?
        .ok_or_else(|| AppError::NotFound("Organization not found".into()))?;
    let organization_slug: String = organization.try_get("", "slug")?;
    let repository = match (repository_name, repository_id) {
        (None, None) => None,
        (Some(logical), Some(id)) => {
            let repo = registry_repository::Entity::find_by_id(id)
                .filter(registry_repository::Column::OrganizationId.eq(organization_id))
                .one(database)
                .await?
                .ok_or_else(|| AppError::NotFound("Registry repository not found".into()))?;
            let project = project::Entity::find_by_id(repo.project_id)
                .filter(project::Column::OrganizationId.eq(organization_id))
                .one(database)
                .await?
                .ok_or_else(|| AppError::NotFound("Project not found".into()))?;
            let expected = format!(
                "{organization_slug}/{}/{}",
                normalize_project_name(&project.name),
                repo.name
            );
            if logical != expected {
                return Err(AppError::NotFound("Registry repository not found".into()));
            }
            Some((logical, repo.id))
        }
        _ => {
            return Err(AppError::BadRequest(
                "Repository name and ID are both required".into(),
            ));
        }
    };
    Ok(ResolvedRegistry {
        organization_id,
        organization_slug,
        storage_revision: registry.storage_revision,
        status: match registry.status {
            ManagedRegistryStatus::Active => "active".into(),
            ManagedRegistryStatus::Maintenance => "maintenance".into(),
        },
        access_key_id: credential.access_key_id,
        secret_access_key: s3_secret.secret_access_key,
        repository_name: repository.as_ref().map(|r| r.0.clone()),
        repository_id: repository.map(|r| r.1),
    })
}

pub async fn get(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
) -> Result<ManagedRegistryView, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let (registry, foundation) = managed_registry::Entity::find_by_id(organization_id)
        .find_also_related(bucket::Entity)
        .one(scoped.connection())
        .await?
        .ok_or_else(|| AppError::NotFound("Managed Registry is not activated".into()))?;
    let foundation =
        foundation.ok_or_else(|| AppError::NotFound("Managed Registry bucket not found".into()))?;
    let view = ManagedRegistryView {
        registry,
        region_id: foundation.region_id,
    };
    scoped.commit().await?;
    Ok(view)
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

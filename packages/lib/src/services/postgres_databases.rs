use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set,
};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::operation::{Operation, bucket_prefix_delete::BucketPrefixDelete};

use crate::entities::{
    credential, organization_region_backup_bucket, postgres_database, postgres_database_branch,
    project, project_environment, region, secret,
};
use crate::error::AppError;
use crate::secrets::Client;
use crate::services::{agent, backups, events, s3_providers::S3ProviderClient};
use crate::tenant::TenantDatabase;

pub struct ServiceContext<'a> {
    pub providers: &'a S3ProviderClient,
    pub secrets: &'a Client,
}

pub struct CreateDatabaseInput {
    pub name: String,
    pub project_id: Uuid,
    pub region_id: Uuid,
    pub backup_retention_days: Option<i32>,
    pub cpu: Option<String>,
    pub ram: Option<String>,
    pub high_availability: bool,
    pub read_replicas: Option<i32>,
    pub autoscaling_enabled: bool,
    pub autoscaling_min_cpu: Option<String>,
    pub autoscaling_max_cpu: Option<String>,
}

pub struct UpdateDatabaseInput {
    pub name: Option<String>,
}

pub struct CreateDatabaseBranchInput {
    pub branch_id: Uuid,
    pub backup_retention_days: Option<i32>,
    pub cpu: Option<String>,
    pub ram: Option<String>,
    pub high_availability: Option<bool>,
    pub read_replicas: Option<i32>,
    pub autoscaling_enabled: Option<bool>,
    pub autoscaling_min_cpu: Option<String>,
    pub autoscaling_max_cpu: Option<String>,
}

pub struct UpdateDatabaseBranchInput {
    pub backup_retention_days: Option<Option<i32>>,
    pub cpu: Option<String>,
    pub ram: Option<String>,
    pub high_availability: Option<bool>,
    pub read_replicas: Option<i32>,
    pub autoscaling_enabled: Option<bool>,
    pub autoscaling_min_cpu: Option<String>,
    pub autoscaling_max_cpu: Option<String>,
}

pub struct CreatedPostgresBranch {
    pub row: postgres_database_branch::Model,
    pub created: bool,
}

pub struct DeletedPostgresBranch {
    pub id: Uuid,
    pub branch_id: Uuid,
    pub access_key_id: String,
}

pub struct DeletedPostgresDatabase {
    pub id: Uuid,
    pub branches: Vec<DeletedPostgresBranch>,
}

pub fn validate_backup_retention_days(retention_days: Option<i32>) -> Result<(), AppError> {
    if retention_days.is_some_and(|days| days <= 0) {
        return Err(AppError::BadRequest(
            "Backup retention must be a positive number of days or disabled".into(),
        ));
    }
    Ok(())
}

fn cpu_cores(value: &str) -> Result<f64, AppError> {
    let cores: f64 = value
        .trim()
        .parse()
        .map_err(|_| AppError::BadRequest("CPU must be a number of cores".into()))?;
    if !cores.is_finite() || cores <= 0.0 || cores > 64.0 {
        return Err(AppError::BadRequest(
            "CPU must be between 0 and 64 cores".into(),
        ));
    }
    Ok(cores)
}

pub fn validate_cpu(value: &str) -> Result<(), AppError> {
    cpu_cores(value).map(|_| ())
}

pub fn validate_ram(value: &str) -> Result<(), AppError> {
    let mib = value
        .trim()
        .strip_suffix("Mi")
        .map(str::to_owned)
        .unwrap_or_else(|| value.trim().to_owned());
    let mib: i64 = mib
        .parse()
        .map_err(|_| AppError::BadRequest("RAM must be a number of mebibytes".into()))?;
    if mib <= 0 || mib > 65536 {
        return Err(AppError::BadRequest(
            "RAM must be between 1 and 65536 MiB".into(),
        ));
    }
    Ok(())
}

pub fn validate_read_replicas(read_replicas: i32) -> Result<(), AppError> {
    if !(0..=64).contains(&read_replicas) {
        return Err(AppError::BadRequest(
            "Read replicas must be between 0 and 64".into(),
        ));
    }
    Ok(())
}

pub fn validate_autoscaling(min_cpu: Option<&str>, max_cpu: Option<&str>) -> Result<(), AppError> {
    let min_cpu = min_cpu.map(cpu_cores).transpose()?;
    let max_cpu = max_cpu.map(cpu_cores).transpose()?;
    if let (Some(min), Some(max)) = (min_cpu, max_cpu)
        && min > max
    {
        return Err(AppError::BadRequest(
            "autoscaling_min_cpu must not exceed autoscaling_max_cpu".into(),
        ));
    }
    Ok(())
}

pub async fn create_database(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
    context: &ServiceContext<'_>,
    mut input: CreateDatabaseInput,
) -> Result<postgres_database::Model, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    input.name = input.name.trim().to_string();
    validate_database_input(&input)?;

    let database_id = Uuid::new_v4();
    let branch_link_id = Uuid::new_v4();
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    verify_project_in_org(tx, input.project_id, organization_id).await?;
    let main_environment = project_environment::Entity::find()
        .filter(project_environment::Column::ProjectId.eq(input.project_id))
        .filter(project_environment::Column::Name.eq("main"))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Main environment not found".into()))?;
    let backup_prefix = format!("postgres/{branch_link_id}/");
    let backup_bucket = backups::ensure_backup_bucket(
        tenant_db,
        organization_id,
        input.region_id,
        context.providers,
        context.secrets,
    )
    .await?;
    let backup_credential_id = backups::provision_access(
        tx,
        context.secrets,
        organization_id,
        backup_bucket.bucket_id,
        "CPPG",
        &backup_prefix,
    )
    .await?;

    let created = postgres_database::ActiveModel {
        id: Set(database_id),
        project_id: Set(input.project_id),
        organization_id: Set(organization_id),
        region_id: Set(input.region_id),
        default_branch_id: Set(None),
        name: Set(input.name.clone()),
    }
    .insert(tx)
    .await?;

    postgres_database_branch::ActiveModel {
        id: Set(branch_link_id),
        database_id: Set(database_id),
        branch_id: Set(main_environment.id),
        organization_id: Set(organization_id),
        backup_retention_days: Set(input.backup_retention_days.or(Some(30))),
        cpu: Set(input.cpu.clone()),
        ram: Set(input.ram.clone()),
        high_availability: Set(input.high_availability),
        read_replicas: Set(input.read_replicas),
        autoscaling_enabled: Set(input.autoscaling_enabled),
        autoscaling_min_cpu: Set(input.autoscaling_min_cpu.clone()),
        autoscaling_max_cpu: Set(input.autoscaling_max_cpu.clone()),
        organization_region_backup_bucket_id: Set(backup_bucket.mapping_id),
        backup_credential_id: Set(backup_credential_id),
    }
    .insert(tx)
    .await?;

    let mut active: postgres_database::ActiveModel = created.into();
    active.default_branch_id = Set(Some(branch_link_id));
    let database = active.update(tx).await?;
    events::record(
        tx,
        organization_id,
        input.project_id,
        "database:created",
        json!({
            "summary": format!("Created database '{}'", input.name),
            "target_id": branch_link_id.to_string(),
            "environment_id": main_environment.id.to_string(),
        }),
        actor_id,
    )
    .await?;
    scoped.commit().await?;
    agent::emit_postgres_branch(
        database.id,
        organization_id,
        main_environment.id,
        branch_link_id,
    )
    .await?;

    Ok(database)
}

pub async fn list_databases(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<
    Vec<(
        postgres_database::Model,
        Vec<postgres_database_branch::Model>,
    )>,
    AppError,
> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    verify_project_in_org(tx, project_id, organization_id).await?;

    let databases = postgres_database::Entity::find()
        .filter(postgres_database::Column::ProjectId.eq(project_id))
        .filter(postgres_database::Column::OrganizationId.eq(organization_id))
        .order_by_asc(postgres_database::Column::Name)
        .all(tx)
        .await?;
    let database_ids = databases
        .iter()
        .map(|database| database.id)
        .collect::<Vec<_>>();
    let branches = if database_ids.is_empty() {
        Vec::new()
    } else {
        postgres_database_branch::Entity::find()
            .filter(postgres_database_branch::Column::DatabaseId.is_in(database_ids))
            .filter(postgres_database_branch::Column::OrganizationId.eq(organization_id))
            .all(tx)
            .await?
    };
    scoped.commit().await?;

    let mut branches_by_database = std::collections::HashMap::new();
    for branch in branches {
        branches_by_database
            .entry(branch.database_id)
            .or_insert_with(Vec::new)
            .push(branch);
    }
    Ok(databases
        .into_iter()
        .map(|database| {
            let branches = branches_by_database
                .remove(&database.id)
                .unwrap_or_default();
            (database, branches)
        })
        .collect())
}

pub async fn get_database(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    database_id: Uuid,
) -> Result<postgres_database::Model, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let database = find_database(scoped.connection(), organization_id, database_id).await?;
    scoped.commit().await?;
    Ok(database)
}

pub async fn update_database(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
    database_id: Uuid,
    input: UpdateDatabaseInput,
) -> Result<postgres_database::Model, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let database = find_database(tx, organization_id, database_id).await?;
    let mut active: postgres_database::ActiveModel = database.clone().into();
    if let Some(name) = input.name {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(AppError::BadRequest("Name is required".into()));
        }
        active.name = Set(name);
    }
    let updated = active.update(tx).await?;
    if let Some(target_id) = updated.default_branch_id {
        events::record(
            tx,
            organization_id,
            updated.project_id,
            "database:updated",
            json!({
                "summary": format!("Updated database '{}'", updated.name),
                "target_id": target_id.to_string(),
            }),
            actor_id,
        )
        .await?;
    }
    scoped.commit().await?;
    Ok(updated)
}

pub async fn delete_database(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
    context: &ServiceContext<'_>,
    database_id: Uuid,
) -> Result<DeletedPostgresDatabase, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let deleted = delete_database_in_transaction(
        scoped.connection(),
        organization_id,
        actor_id,
        context,
        database_id,
    )
    .await?;
    scoped.commit().await?;
    finalize_deleted_databases(context, organization_id, std::slice::from_ref(&deleted)).await?;
    Ok(deleted)
}

pub async fn delete_database_in_transaction(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    actor_id: Uuid,
    context: &ServiceContext<'_>,
    database_id: Uuid,
) -> Result<DeletedPostgresDatabase, AppError> {
    let database = find_database(tx, organization_id, database_id).await?;
    let branches = postgres_database_branch::Entity::find()
        .filter(postgres_database_branch::Column::DatabaseId.eq(database_id))
        .filter(postgres_database_branch::Column::OrganizationId.eq(organization_id))
        .find_also_related(credential::Entity)
        .all(tx)
        .await?;
    let deleted_branches =
        delete_database_branches_in_transaction(tx, organization_id, context, &branches).await?;
    postgres_database::Entity::delete_by_id(database_id)
        .exec(tx)
        .await?;
    if let Some(target_id) = database.default_branch_id {
        events::record(
            tx,
            organization_id,
            database.project_id,
            "database:deleted",
            json!({
                "summary": format!("Deleted database '{}'", database.name),
                "target_id": target_id.to_string(),
            }),
            actor_id,
        )
        .await?;
    }
    Ok(DeletedPostgresDatabase {
        id: database.id,
        branches: deleted_branches,
    })
}

pub async fn list_database_branches(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    database_id: Uuid,
) -> Result<Vec<postgres_database_branch::Model>, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    find_database(tx, organization_id, database_id).await?;
    let branches = postgres_database_branch::Entity::find()
        .filter(postgres_database_branch::Column::DatabaseId.eq(database_id))
        .filter(postgres_database_branch::Column::OrganizationId.eq(organization_id))
        .all(tx)
        .await?;
    scoped.commit().await?;
    Ok(branches)
}

pub async fn update_database_branch(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
    database_id: Uuid,
    branch_id: Uuid,
    input: UpdateDatabaseBranchInput,
) -> Result<postgres_database_branch::Model, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let database = find_database(tx, organization_id, database_id).await?;
    let branch = find_branch(tx, organization_id, database_id, branch_id).await?;
    let active = apply_branch_update(&branch, &database, input)?;
    let updated = active.update(tx).await?;
    events::record(
        tx,
        organization_id,
        database.project_id,
        "database:updated",
        json!({
            "summary": format!("Updated '{}' branch configuration", database.name),
            "target_id": branch.id.to_string(),
            "environment_id": branch_id.to_string(),
        }),
        actor_id,
    )
    .await?;
    scoped.commit().await?;
    agent::emit_postgres_branch(database_id, organization_id, branch_id, updated.id).await?;
    Ok(updated)
}

pub async fn create_database_branch(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
    context: &ServiceContext<'_>,
    database_id: Uuid,
    input: CreateDatabaseBranchInput,
) -> Result<CreatedPostgresBranch, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    validate_backup_retention_days(input.backup_retention_days)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let database = find_database_for_update(tx, organization_id, database_id).await?;
    let environment = project_environment::Entity::find_by_id(input.branch_id)
        .filter(project_environment::Column::ProjectId.eq(database.project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Branch not found in database's project".into()))?;
    let existing = postgres_database_branch::Entity::find()
        .filter(postgres_database_branch::Column::DatabaseId.eq(database_id))
        .filter(postgres_database_branch::Column::BranchId.eq(input.branch_id))
        .filter(postgres_database_branch::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?;
    if let Some(row) = existing {
        scoped.commit().await?;
        return Ok(CreatedPostgresBranch {
            row,
            created: false,
        });
    }

    let defaults = if let Some(default_id) = database.default_branch_id {
        let default = postgres_database_branch::Entity::find_by_id(default_id)
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Default database branch not found".into()))?;
        (
            input.cpu.or(default.cpu),
            input.ram.or(default.ram),
            input.high_availability.or(Some(default.high_availability)),
            input.read_replicas.or(default.read_replicas),
            input
                .autoscaling_enabled
                .or(Some(default.autoscaling_enabled)),
            input.autoscaling_min_cpu.or(default.autoscaling_min_cpu),
            input.autoscaling_max_cpu.or(default.autoscaling_max_cpu),
            input
                .backup_retention_days
                .or(default.backup_retention_days),
        )
    } else {
        (
            input.cpu,
            input.ram,
            input.high_availability,
            input.read_replicas,
            input.autoscaling_enabled,
            input.autoscaling_min_cpu,
            input.autoscaling_max_cpu,
            input.backup_retention_days,
        )
    };
    validate_branch_values(
        defaults.0.as_deref(),
        defaults.1.as_deref(),
        defaults.3,
        defaults.5.as_deref(),
        defaults.6.as_deref(),
    )?;

    let id = Uuid::new_v4();
    let backup_prefix = format!("postgres/{id}/");
    let backup_bucket = backups::ensure_backup_bucket(
        tenant_db,
        organization_id,
        database.region_id,
        context.providers,
        context.secrets,
    )
    .await?;
    let backup_credential_id = backups::provision_access(
        tx,
        context.secrets,
        organization_id,
        backup_bucket.bucket_id,
        "CPPG",
        &backup_prefix,
    )
    .await?;
    let row = postgres_database_branch::ActiveModel {
        id: Set(id),
        database_id: Set(database_id),
        branch_id: Set(environment.id),
        organization_id: Set(organization_id),
        backup_retention_days: Set(defaults.7),
        cpu: Set(defaults.0.clone()),
        ram: Set(defaults.1.clone()),
        high_availability: Set(defaults.2.unwrap_or(false)),
        read_replicas: Set(defaults.3),
        autoscaling_enabled: Set(defaults.4.unwrap_or(false)),
        autoscaling_min_cpu: Set(defaults.5.clone()),
        autoscaling_max_cpu: Set(defaults.6.clone()),
        organization_region_backup_bucket_id: Set(backup_bucket.mapping_id),
        backup_credential_id: Set(backup_credential_id),
    }
    .insert(tx)
    .await?;
    events::record(
        tx,
        organization_id,
        database.project_id,
        "database:linked",
        json!({
            "summary": format!("Linked '{}' to branch '{}'", database.name, environment.name),
            "target_id": row.id.to_string(),
            "environment_id": environment.id.to_string(),
        }),
        actor_id,
    )
    .await?;
    scoped.commit().await?;
    agent::emit_postgres_branch(database_id, organization_id, environment.id, row.id).await?;
    Ok(CreatedPostgresBranch { row, created: true })
}

pub async fn delete_branch(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
    context: &ServiceContext<'_>,
    database_id: Uuid,
    branch_id: Uuid,
) -> Result<DeletedPostgresBranch, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let database = find_database(tx, organization_id, database_id).await?;
    let deleted =
        delete_branch_in_transaction(tx, organization_id, context, &database, branch_id, false)
            .await?;
    events::record(
        tx,
        organization_id,
        database.project_id,
        "database:unlinked",
        json!({
            "summary": format!("Unlinked '{}' from this branch", database.name),
            "target_id": deleted.id.to_string(),
            "environment_id": branch_id.to_string(),
        }),
        actor_id,
    )
    .await?;
    scoped.commit().await?;
    finalize_deleted_branch(context, database_id, organization_id, &deleted).await?;
    Ok(deleted)
}

pub async fn finalize_deleted_databases(
    context: &ServiceContext<'_>,
    organization_id: Uuid,
    deleted_databases: &[DeletedPostgresDatabase],
) -> Result<(), AppError> {
    let access_keys = deleted_databases
        .iter()
        .flat_map(|database| database.branches.iter())
        .map(|branch| branch.access_key_id.clone())
        .collect::<Vec<_>>();
    if let Err(error) = invalidate_caches(context, &access_keys).await {
        tracing::warn!(%error, "postgres database cache invalidation failed after deletion");
    }
    for database in deleted_databases {
        for branch in &database.branches {
            agent::revoke_postgres_branch(
                database.id,
                organization_id,
                branch.branch_id,
                branch.id,
            )
            .await?;
        }
    }
    Ok(())
}

async fn finalize_deleted_branch(
    context: &ServiceContext<'_>,
    database_id: Uuid,
    organization_id: Uuid,
    deleted: &DeletedPostgresBranch,
) -> Result<(), AppError> {
    if let Err(error) =
        invalidate_caches(context, std::slice::from_ref(&deleted.access_key_id)).await
    {
        tracing::warn!(%error, branch_id = %deleted.id, "branch cache invalidation failed after deletion");
    }
    agent::revoke_postgres_branch(database_id, organization_id, deleted.branch_id, deleted.id).await
}

async fn invalidate_caches(
    context: &ServiceContext<'_>,
    access_keys: &[String],
) -> Result<(), AppError> {
    if access_keys.is_empty() {
        return Ok(());
    }
    context
        .providers
        .invalidate_access_token_caches(access_keys)
        .await
}

async fn delete_database_branches_in_transaction(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    _context: &ServiceContext<'_>,
    branches: &[(postgres_database_branch::Model, Option<credential::Model>)],
) -> Result<Vec<DeletedPostgresBranch>, AppError> {
    if branches.is_empty() {
        return Ok(Vec::new());
    }

    let mapping_ids = branches
        .iter()
        .map(|(branch, _)| branch.organization_region_backup_bucket_id)
        .collect::<HashSet<_>>();
    let mappings = organization_region_backup_bucket::Entity::find()
        .filter(
            organization_region_backup_bucket::Column::Id
                .is_in(mapping_ids.iter().copied().collect::<Vec<_>>()),
        )
        .filter(organization_region_backup_bucket::Column::OrganizationId.eq(organization_id))
        .all(tx)
        .await?;
    let mappings_by_id = mappings
        .into_iter()
        .map(|mapping| (mapping.id, mapping))
        .collect::<HashMap<_, _>>();

    let region_ids = mappings_by_id
        .values()
        .map(|mapping| mapping.region_id)
        .collect::<HashSet<_>>();
    let regions = region::Entity::find()
        .filter(region::Column::Id.is_in(region_ids.iter().copied().collect::<Vec<_>>()))
        .all(tx)
        .await?;
    let regions_by_id = regions
        .into_iter()
        .map(|region| (region.id, region))
        .collect::<HashMap<_, _>>();

    let mut locked_regions = HashSet::new();
    let mut jobs = Vec::with_capacity(branches.len());
    let mut credential_ids = Vec::with_capacity(branches.len());
    let mut secret_ids = Vec::with_capacity(branches.len());
    let mut deleted = Vec::with_capacity(branches.len());

    for (branch, backup_credential) in branches {
        let backup_credential = backup_credential.as_ref().ok_or_else(|| {
            AppError::Internal(format!(
                "Backup credential {} not found for database branch {}",
                branch.backup_credential_id, branch.id
            ))
        })?;
        let mapping = mappings_by_id
            .get(&branch.organization_region_backup_bucket_id)
            .ok_or_else(|| AppError::NotFound("Postgres backup bucket mapping not found".into()))?;
        let region = regions_by_id
            .get(&mapping.region_id)
            .ok_or_else(|| AppError::NotFound("Postgres backup region not found".into()))?;
        let provider_id = region.s3_provider_id.ok_or_else(|| {
            AppError::Conflict("Postgres backup region has no S3 provider".into())
        })?;

        if locked_regions.insert(mapping.region_id) {
            backups::lock_region(tx, organization_id, mapping.region_id).await?;
        }
        jobs.push((branch.id, mapping.bucket_id, provider_id));
        credential_ids.push(backup_credential.id);
        secret_ids.push(backup_credential.secret_id);
        deleted.push(DeletedPostgresBranch {
            id: branch.id,
            branch_id: branch.branch_id,
            access_key_id: backup_credential.access_key_id.clone(),
        });
    }

    postgres_database_branch::Entity::delete_many()
        .filter(
            postgres_database_branch::Column::Id.is_in(
                branches
                    .iter()
                    .map(|(branch, _)| branch.id)
                    .collect::<Vec<_>>(),
            ),
        )
        .filter(postgres_database_branch::Column::OrganizationId.eq(organization_id))
        .exec(tx)
        .await?;
    credential::Entity::delete_many()
        .filter(credential::Column::Id.is_in(credential_ids))
        .exec(tx)
        .await?;
    secret::Entity::delete_many()
        .filter(secret::Column::Id.is_in(secret_ids))
        .exec(tx)
        .await?;
    Operation::<BucketPrefixDelete>::new_many(
        tx,
        organization_id,
        jobs.into_iter().map(|(branch_id, bucket_id, provider_id)| {
            (
                branch_id.to_string(),
                BucketPrefixDelete {
                    provider_id,
                    bucket_id,
                    prefix: format!("postgres/{branch_id}/"),
                },
            )
        }),
    )
    .await?;

    for mapping in mappings_by_id.values() {
        let remaining_branch = postgres_database_branch::Entity::find()
            .filter(
                postgres_database_branch::Column::OrganizationRegionBackupBucketId.eq(mapping.id),
            )
            .filter(postgres_database_branch::Column::OrganizationId.eq(organization_id))
            .one(tx)
            .await?;
        if remaining_branch.is_none() {
            organization_region_backup_bucket::Entity::delete_by_id(mapping.id)
                .exec(tx)
                .await?;
            crate::buckets::delete_foundation(tx, mapping.bucket_id).await?;
        }
    }

    Ok(deleted)
}

async fn delete_branch_in_transaction(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    _context: &ServiceContext<'_>,
    database: &postgres_database::Model,
    branch_id: Uuid,
    allow_default: bool,
) -> Result<DeletedPostgresBranch, AppError> {
    let (branch, backup_credential) = postgres_database_branch::Entity::find()
        .filter(postgres_database_branch::Column::DatabaseId.eq(database.id))
        .filter(postgres_database_branch::Column::BranchId.eq(branch_id))
        .filter(postgres_database_branch::Column::OrganizationId.eq(organization_id))
        .find_also_related(credential::Entity)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Database branch link not found".into()))?;
    if !allow_default && database.default_branch_id == Some(branch.id) {
        return Err(AppError::Conflict(
            "Cannot delete the default database branch".into(),
        ));
    }
    let backup_credential = backup_credential.ok_or_else(|| {
        AppError::Internal(format!(
            "Backup credential {} not found for database branch {}",
            branch.backup_credential_id, branch.id
        ))
    })?;
    let mapping = organization_region_backup_bucket::Entity::find_by_id(
        branch.organization_region_backup_bucket_id,
    )
    .filter(organization_region_backup_bucket::Column::OrganizationId.eq(organization_id))
    .one(tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Postgres backup bucket mapping not found".into()))?;
    let region = region::Entity::find_by_id(mapping.region_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Postgres backup region not found".into()))?;
    let provider_id = region
        .s3_provider_id
        .ok_or_else(|| AppError::Conflict("Postgres backup region has no S3 provider".into()))?;
    backups::lock_region(tx, organization_id, mapping.region_id).await?;

    postgres_database_branch::Entity::delete_by_id(branch.id)
        .exec(tx)
        .await?;
    credential::Entity::delete_by_id(backup_credential.id)
        .exec(tx)
        .await?;
    secret::Entity::delete_by_id(backup_credential.secret_id)
        .exec(tx)
        .await?;
    Operation::<BucketPrefixDelete>::new(
        tx,
        organization_id,
        branch.id.to_string(),
        BucketPrefixDelete {
            provider_id,
            bucket_id: mapping.bucket_id,
            prefix: format!("postgres/{}/", branch.id),
        },
    )
    .await?;
    let remaining_branch = postgres_database_branch::Entity::find()
        .filter(postgres_database_branch::Column::OrganizationRegionBackupBucketId.eq(mapping.id))
        .filter(postgres_database_branch::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?;
    if remaining_branch.is_none() {
        organization_region_backup_bucket::Entity::delete_by_id(mapping.id)
            .exec(tx)
            .await?;
        crate::buckets::delete_foundation(tx, mapping.bucket_id).await?;
    }
    Ok(DeletedPostgresBranch {
        id: branch.id,
        branch_id: branch.branch_id,
        access_key_id: backup_credential.access_key_id,
    })
}

async fn find_database(
    tx: &impl ConnectionTrait,
    organization_id: Uuid,
    database_id: Uuid,
) -> Result<postgres_database::Model, AppError> {
    let database = postgres_database::Entity::find_by_id(database_id)
        .filter(postgres_database::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Postgres database not found".into()))?;
    verify_project_in_org(tx, database.project_id, organization_id).await?;
    Ok(database)
}

async fn find_database_for_update(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    database_id: Uuid,
) -> Result<postgres_database::Model, AppError> {
    let database = postgres_database::Entity::find_by_id(database_id)
        .filter(postgres_database::Column::OrganizationId.eq(organization_id))
        .lock_exclusive()
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Postgres database not found".into()))?;
    verify_project_in_org(tx, database.project_id, organization_id).await?;
    Ok(database)
}

async fn find_branch(
    tx: &impl ConnectionTrait,
    organization_id: Uuid,
    database_id: Uuid,
    branch_id: Uuid,
) -> Result<postgres_database_branch::Model, AppError> {
    postgres_database_branch::Entity::find()
        .filter(postgres_database_branch::Column::DatabaseId.eq(database_id))
        .filter(postgres_database_branch::Column::BranchId.eq(branch_id))
        .filter(postgres_database_branch::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Database branch link not found".into()))
}

fn validate_database_input(input: &CreateDatabaseInput) -> Result<(), AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::BadRequest("Name is required".into()));
    }
    validate_branch_values(
        input.cpu.as_deref(),
        input.ram.as_deref(),
        input.read_replicas,
        input.autoscaling_min_cpu.as_deref(),
        input.autoscaling_max_cpu.as_deref(),
    )?;
    validate_backup_retention_days(input.backup_retention_days)?;
    Ok(())
}

fn validate_branch_values(
    cpu: Option<&str>,
    ram: Option<&str>,
    read_replicas: Option<i32>,
    autoscaling_min_cpu: Option<&str>,
    autoscaling_max_cpu: Option<&str>,
) -> Result<(), AppError> {
    if let Some(cpu) = cpu {
        validate_cpu(cpu)?;
    }
    if let Some(ram) = ram {
        validate_ram(ram)?;
    }
    if let Some(read_replicas) = read_replicas {
        validate_read_replicas(read_replicas)?;
    }
    validate_autoscaling(autoscaling_min_cpu, autoscaling_max_cpu)
}

fn apply_branch_update(
    branch: &postgres_database_branch::Model,
    database: &postgres_database::Model,
    input: UpdateDatabaseBranchInput,
) -> Result<postgres_database_branch::ActiveModel, AppError> {
    let mut active: postgres_database_branch::ActiveModel = branch.clone().into();
    if let Some(value) = input.backup_retention_days {
        validate_backup_retention_days(value)?;
        if value.is_none() && database.default_branch_id == Some(branch.id) {
            return Err(AppError::Conflict(
                "The default database branch must retain backups".into(),
            ));
        }
        active.backup_retention_days = Set(value);
    }
    validate_branch_values(
        input.cpu.as_deref().or(branch.cpu.as_deref()),
        input.ram.as_deref().or(branch.ram.as_deref()),
        input.read_replicas.or(branch.read_replicas),
        input
            .autoscaling_min_cpu
            .as_deref()
            .or(branch.autoscaling_min_cpu.as_deref()),
        input
            .autoscaling_max_cpu
            .as_deref()
            .or(branch.autoscaling_max_cpu.as_deref()),
    )?;
    if input.cpu.is_some() {
        active.cpu = Set(input.cpu);
    }
    if input.ram.is_some() {
        active.ram = Set(input.ram);
    }
    if let Some(value) = input.high_availability {
        active.high_availability = Set(value);
    }
    if input.read_replicas.is_some() {
        active.read_replicas = Set(input.read_replicas);
    }
    if let Some(value) = input.autoscaling_enabled {
        active.autoscaling_enabled = Set(value);
    }
    if input.autoscaling_min_cpu.is_some() {
        active.autoscaling_min_cpu = Set(input.autoscaling_min_cpu);
    }
    if input.autoscaling_max_cpu.is_some() {
        active.autoscaling_max_cpu = Set(input.autoscaling_max_cpu);
    }
    Ok(active)
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
    tx: &impl ConnectionTrait,
    project_id: Uuid,
    organization_id: Uuid,
) -> Result<(), AppError> {
    let exists = project::Entity::find()
        .filter(project::Column::Id.eq(project_id))
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .is_some();
    if exists {
        Ok(())
    } else {
        Err(AppError::NotFound(
            "Project not found in this organization".into(),
        ))
    }
}

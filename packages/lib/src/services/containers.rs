use crate::{
    entities::{container, project, project_environment, project_timeline},
    error::AppError,
    manifest::{ContainerConfig, RevisionManifest},
    secrets::Client,
    services::{agent, events, external_registries, images, postgres_databases, revisions},
    tenant::TenantDatabase,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter,
    QueryOrder, Set,
};
use uuid::Uuid;

pub fn verify_org_access(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
) -> Result<(), AppError> {
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

pub fn validate_replica_count(replica_count: i32) -> Result<(), AppError> {
    if !(1..=100).contains(&replica_count) {
        return Err(AppError::BadRequest(
            "Replica count must be between 1 and 100".into(),
        ));
    }
    Ok(())
}

pub fn validate_resources(cpu: &Option<String>, memory: &Option<String>) -> Result<(), AppError> {
    match (cpu, memory) {
        (None, None) => Ok(()),
        (Some(cpu), Some(memory)) => {
            postgres_databases::validate_cpu(cpu)?;
            postgres_databases::validate_ram(memory)?;
            Ok(())
        }
        _ => Err(AppError::BadRequest(
            "Resources require both cpu and memory".into(),
        )),
    }
}

pub fn validate_port(port: i32) -> Result<(), AppError> {
    if !(1..=65535).contains(&port) {
        return Err(AppError::BadRequest(
            "Port must be between 1 and 65535".into(),
        ));
    }
    Ok(())
}

pub struct UpdateInput {
    pub container_id: Uuid,
    pub environment_id: Uuid,
    pub timeline_id: Uuid,
    pub name: Option<String>,
    pub image: Option<String>,
    pub public: Option<bool>,
    pub replica_count: Option<i32>,
    pub port: Option<i32>,
    pub env: Option<serde_json::Value>,
    pub cpu: Option<Option<String>>,
    pub memory: Option<Option<String>>,
    pub external_registry_id: Option<Option<Uuid>>,
    pub health_check: Option<serde_json::Value>,
    pub auto_deploy: bool,
    pub redeploy: bool,
    pub timeline_summary: String,
}

pub struct DeleteInput {
    pub container_id: Uuid,
    pub environment_id: Uuid,
    pub timeline_id: Uuid,
    pub deploy: bool,
}

fn image_resolution_needed(
    image: Option<&str>,
    registry_id: Option<Option<Uuid>>,
    redeploy: bool,
    base: &ContainerConfig,
) -> bool {
    redeploy
        || image.is_some_and(|image| image.trim() != base.image)
        || registry_id.is_some_and(|id| id != base.external_registry_id)
}

pub struct CreateInput {
    pub project_id: Uuid,
    pub environment_id: Uuid,
    pub name: String,
    pub image: String,
    pub public: bool,
    pub replica_count: i32,
    pub port: Option<i32>,
    pub env: Option<serde_json::Value>,
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub external_registry_id: Option<Uuid>,
    pub health_check: Option<serde_json::Value>,
    pub auto_deploy: bool,
    pub region_id: Uuid,
}

pub struct Created {
    pub container: container::Model,
    pub manifest: RevisionManifest,
    pub revision_id: Uuid,
}

pub struct ListInput {
    pub project_id: Option<Uuid>,
    pub environment_id: Option<Uuid>,
    pub timeline_id: Option<Uuid>,
}
pub struct Listed {
    pub container: container::Model,
    pub manifest: RevisionManifest,
    pub revision_id: Uuid,
}
pub struct GetInput {
    pub container_id: Uuid,
    pub environment_id: Option<Uuid>,
    pub timeline_id: Option<Uuid>,
}

pub async fn list(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    input: ListInput,
) -> Result<Vec<Listed>, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let project_id = if let Some(environment_id) = input.environment_id {
        Some(
            get_environment(tx, environment_id, organization_id, input.project_id)
                .await?
                .project_id,
        )
    } else if let Some(id) = input.project_id {
        Some(id)
    } else if let Some(id) = input.timeline_id {
        Some(
            timeline_manifest(tx, id, organization_id, None)
                .await?
                .0
                .project_id,
        )
    } else {
        None
    };
    let result = if let Some(project_id) = project_id {
        let environment =
            resolve_environment(tx, organization_id, project_id, input.environment_id).await?;
        let revision_id = input.timeline_id.unwrap_or(environment.draft_timeline);
        let (_, manifest) =
            timeline_manifest(tx, revision_id, organization_id, Some(project_id)).await?;
        manifest_rows(tx, &manifest, revision_id).await?
    } else {
        #[derive(FromQueryResult)]
        struct DefaultManifest {
            project_id: Uuid,
            revision_id: Uuid,
            schema_version: i32,
            configuration: serde_json::Value,
        }
        let defaults = DefaultManifest::find_by_statement(sea_orm::Statement::from_sql_and_values(sea_orm::DatabaseBackend::Postgres, "SELECT p.id AS project_id, t.id AS revision_id, m.schema_version, m.configuration FROM project p JOIN project_environment e ON e.id = p.default_environment_id AND e.project_id = p.id JOIN project_timeline t ON t.id = e.draft_timeline AND t.project_id = p.id JOIN project_revision_manifest m ON m.id = t.manifest_id WHERE p.organization_id = $1", [organization_id.into()])).all(tx).await?;
        let containers = container::Entity::find()
            .filter(container::Column::OrganizationId.eq(organization_id))
            .all(tx)
            .await?;
        let by_id = containers
            .iter()
            .map(|c| (c.id, c))
            .collect::<std::collections::HashMap<_, _>>();
        let mut rows = Vec::new();
        for row in defaults {
            let manifest =
                RevisionManifest::from_json_value(&row.configuration, row.schema_version)?;
            for id in manifest.containers.keys() {
                if let Some(c) = by_id.get(id) {
                    if c.project_id == row.project_id {
                        rows.push(Listed {
                            container: (*c).clone(),
                            manifest: manifest.clone(),
                            revision_id: row.revision_id,
                        });
                    }
                }
            }
        }
        rows.sort_by(|a, b| {
            let an = a
                .manifest
                .containers
                .get(&a.container.id)
                .map(|c| c.name.as_str())
                .unwrap_or(a.container.name.as_str());
            let bn = b
                .manifest
                .containers
                .get(&b.container.id)
                .map(|c| c.name.as_str())
                .unwrap_or(b.container.name.as_str());
            an.cmp(bn)
        });
        rows
    };
    scoped.commit().await?;
    Ok(result)
}

pub async fn get(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    input: GetInput,
) -> Result<Listed, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let c = container::Entity::find()
        .filter(container::Column::Id.eq(input.container_id))
        .filter(container::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Container not found".into()))?;
    let revision_id = if let Some(id) = input.timeline_id {
        if input.environment_id.is_none() {
            return Err(AppError::BadRequest(
                "environment_id is required with timeline_id".into(),
            ));
        }
        get_environment(
            tx,
            input.environment_id.unwrap(),
            organization_id,
            Some(c.project_id),
        )
        .await?;
        id
    } else {
        resolve_environment(tx, organization_id, c.project_id, input.environment_id)
            .await?
            .draft_timeline
    };
    let (timeline, manifest) =
        timeline_manifest(tx, revision_id, organization_id, Some(c.project_id)).await?;
    if !manifest.containers.contains_key(&c.id) {
        return Err(AppError::NotFound(
            "Container not present in timeline revision".into(),
        ));
    }
    scoped.commit().await?;
    Ok(Listed {
        container: c,
        manifest,
        revision_id: timeline.id,
    })
}

pub async fn delete_project_in_transaction(
    tx: &sea_orm::DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
) -> Result<(), AppError> {
    container::Entity::delete_many()
        .filter(container::Column::ProjectId.eq(project_id))
        .filter(container::Column::OrganizationId.eq(organization_id))
        .exec(tx)
        .await?;
    Ok(())
}

pub async fn create(
    tenant_db: &TenantDatabase,
    identity_db: &sea_orm::DatabaseConnection,
    secrets: &Client,
    registry_token_ttl_seconds: u64,
    organization_id: Uuid,
    actor_id: Uuid,
    input: CreateInput,
) -> Result<Created, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let name = input.name.trim().to_owned();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name is required".into()));
    }
    let image = input.image.trim().to_owned();
    if image.is_empty() {
        return Err(AppError::BadRequest("Image is required".into()));
    }
    if input.region_id.is_nil() {
        return Err(AppError::BadRequest("Region is required".into()));
    }
    validate_replica_count(input.replica_count)?;
    if let Some(port) = input.port {
        validate_port(port)?;
    }
    validate_resources(&input.cpu, &input.memory)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let environment = get_environment(
        tx,
        input.environment_id,
        organization_id,
        Some(input.project_id),
    )
    .await?;
    let registry = match input.external_registry_id {
        Some(id) => Some(external_registries::find(tx, organization_id, id).await?),
        None => None,
    };
    let resolved_image = images::resolve_image(
        &image,
        organization_id,
        registry.as_ref(),
        Some(&images::ImageContext {
            identity_db,
            secrets,
            registry_token_ttl_seconds,
        }),
    )
    .await?;
    let id = Uuid::new_v4();
    let created = container::ActiveModel {
        id: Set(id),
        project_id: Set(input.project_id),
        organization_id: Set(organization_id),
        name: Set(name.clone()),
        region_id: Set(input.region_id),
        created_at: Set(Utc::now().fixed_offset()),
        updated_at: Set(Utc::now().fixed_offset()),
    }
    .insert(tx)
    .await?;
    let (_, mut manifest) = draft_manifest(tx, &environment).await?;
    manifest.containers.insert(
        id,
        ContainerConfig {
            name: name.clone(),
            region_id: input.region_id,
            image,
            resolved_image,
            external_registry_id: registry.as_ref().map(|r| r.id),
            replica_count: input.replica_count,
            port: input.port,
            public: input.public,
            cpu: input.cpu,
            memory: input.memory,
            health_check: input.health_check,
            env: input.env,
        },
    );
    let revision = revisions::create_revision(
        tx,
        &environment,
        &manifest,
        Some(format!("Created container '{name}'")),
        input.auto_deploy,
    )
    .await?;
    events::record(tx, organization_id, input.project_id, "container:created", serde_json::json!({"summary": format!("Created container '{name}'"), "target_id": id.to_string(), "environment_id": environment.id.to_string()}), actor_id).await?;
    scoped.commit().await?;
    if input.auto_deploy {
        agent::emit_compute(
            environment.project_id,
            organization_id,
            environment.id,
            revision.id,
        )
        .await?;
    }
    Ok(Created {
        container: created,
        manifest,
        revision_id: revision.id,
    })
}

pub async fn update(
    tenant_db: &TenantDatabase,
    identity_db: &sea_orm::DatabaseConnection,
    secrets: &Client,
    registry_token_ttl_seconds: u64,
    organization_id: Uuid,
    actor_id: Uuid,
    input: UpdateInput,
) -> Result<Listed, AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let c = container::Entity::find()
        .filter(container::Column::Id.eq(input.container_id))
        .filter(container::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Container not found".into()))?;
    let environment = get_environment(
        tx,
        input.environment_id,
        organization_id,
        Some(c.project_id),
    )
    .await?;
    if input.timeline_id != environment.draft_timeline {
        return Err(AppError::Conflict(
            "Select the environment's draft revision before changing its configuration".into(),
        ));
    }
    let next_name = input.name.as_ref().map(|v| v.trim().to_owned());
    if next_name.as_deref() == Some("") {
        return Err(AppError::BadRequest("Name is required".into()));
    }
    if let Some(v) = input.replica_count {
        validate_replica_count(v)?;
    }
    if let Some(v) = input.port {
        validate_port(v)?;
    }
    let (draft, manifest) =
        timeline_manifest(tx, input.timeline_id, organization_id, Some(c.project_id)).await?;
    let base =
        manifest.containers.get(&c.id).cloned().ok_or_else(|| {
            AppError::NotFound("Container not present in timeline revision".into())
        })?;
    let revise = input.image.is_some()
        || input.public.is_some()
        || input.replica_count.is_some()
        || input.port.is_some()
        || input.env.is_some()
        || input.cpu.is_some()
        || input.memory.is_some()
        || input.external_registry_id.is_some()
        || input.health_check.is_some()
        || next_name.is_some()
        || input.redeploy;
    let mut output = manifest.clone();
    let mut revision_id = draft.id;
    let mut compute = None;
    if revise {
        let cpu = input.cpu.clone().unwrap_or_else(|| base.cpu.clone());
        let memory = input.memory.clone().unwrap_or_else(|| base.memory.clone());
        validate_resources(&cpu, &memory)?;
        let image = input
            .image
            .as_deref()
            .map(str::trim)
            .unwrap_or(&base.image)
            .to_owned();
        if image.is_empty() {
            return Err(AppError::BadRequest("Image is required".into()));
        }
        let registry_id = input
            .external_registry_id
            .unwrap_or(base.external_registry_id);
        let resolve = image_resolution_needed(
            input.image.as_deref(),
            input.external_registry_id,
            input.redeploy,
            &base,
        );
        let (resolved, external_registry_id) = if resolve {
            let registry = match registry_id {
                Some(id) => Some(external_registries::find(tx, organization_id, id).await?),
                None => None,
            };
            let resolved = images::resolve_image(
                &image,
                organization_id,
                registry.as_ref(),
                Some(&images::ImageContext {
                    identity_db,
                    secrets,
                    registry_token_ttl_seconds,
                }),
            )
            .await?;
            (resolved, registry.map(|r| r.id))
        } else {
            (base.resolved_image.clone(), base.external_registry_id)
        };
        output.containers.insert(
            c.id,
            ContainerConfig {
                name: next_name.unwrap_or(base.name),
                region_id: base.region_id,
                image,
                resolved_image: resolved,
                external_registry_id,
                replica_count: input.replica_count.unwrap_or(base.replica_count),
                port: input.port.or(base.port),
                public: input.public.unwrap_or(base.public),
                cpu,
                memory,
                health_check: input.health_check.or(base.health_check),
                env: input.env.or(base.env),
            },
        );
        let revision = revisions::create_revision(
            tx,
            &environment,
            &output,
            Some(input.timeline_summary),
            input.auto_deploy,
        )
        .await?;
        revision_id = revision.id;
        if input.auto_deploy {
            compute = Some(revision.id);
        }
    }
    events::record(tx,organization_id,c.project_id,"container:updated",serde_json::json!({"summary":format!("Updated container '{}'",c.name),"target_id":c.id.to_string(),"environment_id":environment.id.to_string()}),actor_id).await?;
    scoped.commit().await?;
    if let Some(revision_id) = compute {
        agent::emit_compute(
            environment.project_id,
            organization_id,
            environment.id,
            revision_id,
        )
        .await?;
    }
    Ok(Listed {
        container: c,
        manifest: output,
        revision_id,
    })
}

pub async fn delete(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    actor_id: Uuid,
    input: DeleteInput,
) -> Result<(), AppError> {
    verify_org_access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let c = container::Entity::find()
        .filter(container::Column::Id.eq(input.container_id))
        .filter(container::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Container not found".into()))?;
    let environment = get_environment(
        tx,
        input.environment_id,
        organization_id,
        Some(c.project_id),
    )
    .await?;
    if input.timeline_id != environment.draft_timeline {
        return Err(AppError::Conflict(
            "Switch to the draft revision before removing a container".into(),
        ));
    }
    let (_, mut manifest) = draft_manifest(tx, &environment).await?;
    manifest.containers.remove(&input.container_id);
    let revision = revisions::create_revision(
        tx,
        &environment,
        &manifest,
        Some(format!("Removed container '{}'", c.name)),
        input.deploy,
    )
    .await?;
    events::record(tx, organization_id, c.project_id, "container:removed", serde_json::json!({"summary": format!("Removed container '{}'", c.name), "target_id": input.container_id.to_string(), "environment_id": environment.id.to_string()}), actor_id).await?;
    scoped.commit().await?;
    if input.deploy {
        agent::emit_compute(
            environment.project_id,
            organization_id,
            environment.id,
            revision.id,
        )
        .await?;
    }
    Ok(())
}

async fn get_environment(
    tx: &impl ConnectionTrait,
    id: Uuid,
    organization_id: Uuid,
    project_id: Option<Uuid>,
) -> Result<project_environment::Model, AppError> {
    let mut query = project_environment::Entity::find()
        .filter(project_environment::Column::Id.eq(id))
        .filter(project_environment::Column::OrganizationId.eq(organization_id));
    if let Some(project_id) = project_id {
        query = query.filter(project_environment::Column::ProjectId.eq(project_id));
    }
    query
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Environment not found".into()))
}

async fn resolve_environment(
    tx: &impl ConnectionTrait,
    organization_id: Uuid,
    project_id: Uuid,
    environment_id: Option<Uuid>,
) -> Result<project_environment::Model, AppError> {
    if let Some(id) = environment_id {
        return get_environment(tx, id, organization_id, Some(project_id)).await;
    }
    let id = project::Entity::find_by_id(project_id)
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .and_then(|p| p.default_environment_id)
        .ok_or_else(|| AppError::NotFound("Project has no default environment".into()))?;
    get_environment(tx, id, organization_id, Some(project_id)).await
}

async fn timeline_manifest(
    tx: &impl ConnectionTrait,
    timeline_id: Uuid,
    organization_id: Uuid,
    project_id: Option<Uuid>,
) -> Result<(project_timeline::Model, RevisionManifest), AppError> {
    let mut q = project_timeline::Entity::find()
        .filter(project_timeline::Column::Id.eq(timeline_id))
        .filter(project_timeline::Column::OrganizationId.eq(organization_id));
    if let Some(id) = project_id {
        q = q.filter(project_timeline::Column::ProjectId.eq(id));
    }
    let timeline = q
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Timeline revision is not in this project".into()))?;
    Ok((
        timeline.clone(),
        revisions::load_manifest(tx, timeline.manifest_id).await?,
    ))
}

async fn manifest_rows(
    tx: &impl ConnectionTrait,
    manifest: &RevisionManifest,
    revision_id: Uuid,
) -> Result<Vec<Listed>, AppError> {
    if manifest.containers.is_empty() {
        return Ok(Vec::new());
    }
    let ids = manifest.containers.keys().copied().collect::<Vec<_>>();
    let rows = container::Entity::find()
        .filter(container::Column::Id.is_in(ids))
        .order_by_asc(container::Column::Name)
        .all(tx)
        .await?;
    let mut result = rows
        .into_iter()
        .filter(|c| manifest.containers.contains_key(&c.id))
        .map(|c| Listed {
            container: c,
            manifest: manifest.clone(),
            revision_id,
        })
        .collect::<Vec<_>>();
    result.sort_by(|a, b| {
        let an = a
            .manifest
            .containers
            .get(&a.container.id)
            .map(|c| c.name.as_str())
            .unwrap_or(a.container.name.as_str());
        let bn = b
            .manifest
            .containers
            .get(&b.container.id)
            .map(|c| c.name.as_str())
            .unwrap_or(b.container.name.as_str());
        an.cmp(bn).then(a.container.id.cmp(&b.container.id))
    });
    Ok(result)
}

async fn draft_manifest(
    tx: &impl ConnectionTrait,
    environment: &project_environment::Model,
) -> Result<(project_timeline::Model, RevisionManifest), AppError> {
    let timeline = project_timeline::Entity::find_by_id(environment.draft_timeline)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Timeline revision is not in this project".into()))?;
    let manifest = revisions::load_manifest(tx, timeline.manifest_id).await?;
    Ok((timeline, manifest))
}

#[cfg(test)]
mod tests {
    use super::{ContainerConfig, image_resolution_needed};
    use uuid::Uuid;

    #[test]
    fn redeploy_or_registry_change_resolves_image() {
        let registry = Uuid::new_v4();
        let base = ContainerConfig {
            name: "web".into(),
            region_id: Uuid::nil(),
            image: "web:latest".into(),
            resolved_image: "web@sha256:old".into(),
            external_registry_id: Some(registry),
            replica_count: 1,
            port: None,
            public: false,
            cpu: None,
            memory: None,
            health_check: None,
            env: None,
        };
        assert!(!image_resolution_needed(
            Some(" web:latest "),
            Some(Some(registry)),
            false,
            &base
        ));
        assert!(image_resolution_needed(None, None, true, &base));
        assert!(image_resolution_needed(None, Some(None), false, &base));
    }
}

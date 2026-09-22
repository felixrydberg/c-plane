use super::*;
use crate::handlers::databases::verify_project_in_org;
use crate::models::entities::project_revision_manifest;
use crate::models::manifest::ContainerConfig;
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QuerySelect,
    sea_query::Expr,
};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap, HashSet};

const HISTORY_DEFAULT_LIMIT: usize = 10;
const HISTORY_MAX_LIMIT: usize = 50;

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContainerHistoryBaseline {
    Initial,
    EarliestAvailable,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ContainerChangeType {
    Added,
    Changed,
    Removed,
}

#[derive(Serialize, ToSchema)]
pub struct ContainerHistoryChange {
    pub field: String,
    pub change_type: ContainerChangeType,
    pub before: Value,
    pub after: Value,
}

#[derive(Serialize, ToSchema)]
pub struct ContainerHistoryEntry {
    pub revision_id: Uuid,
    pub revision_number: i32,
    pub created_at: String,
    pub baseline: Option<ContainerHistoryBaseline>,
    pub changes: Vec<ContainerHistoryChange>,
}

#[derive(Serialize, ToSchema)]
pub struct ContainerHistoryPage {
    pub data: Vec<ContainerHistoryEntry>,
    pub next_cursor: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ContainerHistoryQuery {
    pub environment_id: Uuid,
    pub timeline_id: Uuid,
    pub limit: Option<u64>,
    pub cursor: Option<String>,
}

fn history_limit(limit: Option<u64>) -> usize {
    limit
        .unwrap_or(HISTORY_DEFAULT_LIMIT as u64)
        .clamp(1, HISTORY_MAX_LIMIT as u64) as usize
}

fn encode_cursor(timeline_id: Uuid) -> String {
    URL_SAFE_NO_PAD.encode(timeline_id.as_bytes())
}

fn decode_cursor(cursor: &str) -> Result<Uuid, AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor)
        .map_err(|_| AppError::BadRequest("Invalid history cursor".into()))?;
    let bytes: [u8; 16] = bytes
        .try_into()
        .map_err(|_| AppError::BadRequest("Invalid history cursor".into()))?;
    Ok(Uuid::from_bytes(bytes))
}

fn config_value(config: Option<&ContainerConfig>) -> Value {
    match config {
        Some(config) => json!({
            "name": config.name,
            "region_id": config.region_id,
            "image": config.image,
            "resolved_image": config.resolved_image,
            "external_registry_id": config.external_registry_id,
            "replica_count": config.replica_count,
            "port": config.port,
            "public": config.public,
            "cpu": config.cpu,
            "memory": config.memory,
            "health_check": config.health_check,
        }),
        None => Value::Null,
    }
}

fn config_changes(
    before: Option<&ContainerConfig>,
    after: Option<&ContainerConfig>,
) -> Vec<ContainerHistoryChange> {
    if before == after {
        return Vec::new();
    }
    let old = config_value(before);
    let new = config_value(after);
    let mut changes = Vec::new();
    let mut push_change =
        |field: String, before: Option<&Value>, after: Option<&Value>, masked: bool| {
            if before == after {
                return;
            }
            let change_type = if before.is_none() {
                ContainerChangeType::Added
            } else if after.is_none() {
                ContainerChangeType::Removed
            } else {
                ContainerChangeType::Changed
            };
            let display = |value: Option<&Value>| {
                if masked && value.is_some() {
                    json!("••••••••")
                } else {
                    value.cloned().unwrap_or(Value::Null)
                }
            };
            changes.push(ContainerHistoryChange {
                field,
                change_type,
                before: display(before),
                after: display(after),
            });
        };
    for field in [
        "name",
        "region_id",
        "image",
        "resolved_image",
        "external_registry_id",
        "replica_count",
        "port",
        "public",
        "cpu",
        "memory",
        "health_check",
    ] {
        push_change(
            field.into(),
            old.get(field).filter(|v| !v.is_null()),
            new.get(field).filter(|v| !v.is_null()),
            false,
        );
    }
    let old_env = before
        .and_then(|config| config.env.as_ref())
        .and_then(Value::as_object);
    let new_env = after
        .and_then(|config| config.env.as_ref())
        .and_then(Value::as_object);
    let keys: BTreeSet<_> = old_env
        .into_iter()
        .chain(new_env)
        .flat_map(|env| env.keys())
        .collect();
    for key in keys {
        push_change(
            format!("env.{key}"),
            old_env.and_then(|env| env.get(key)),
            new_env.and_then(|env| env.get(key)),
            true,
        );
    }
    changes
}

fn entry(
    revision: &project_timeline::Model,
    changes: Vec<ContainerHistoryChange>,
    baseline: Option<ContainerHistoryBaseline>,
) -> ContainerHistoryEntry {
    ContainerHistoryEntry {
        revision_id: revision.id,
        revision_number: revision.timeline,
        created_at: revision.created_at.to_string(),
        baseline,
        changes,
    }
}

fn build_history(
    steps: &[project_timeline::Model],
    configs: &[Option<ContainerConfig>],
    more: bool,
    page_limit: usize,
) -> (Vec<ContainerHistoryEntry>, Option<String>) {
    let emitted = if more {
        page_limit.min(steps.len())
    } else {
        steps.len()
    };
    let mut data = Vec::new();
    for index in 0..emitted {
        let after = configs.get(index).cloned().flatten();
        if index + 1 == steps.len() {
            if let Some(after) = after.as_ref() {
                let baseline = if steps[index].timeline == 1 {
                    ContainerHistoryBaseline::Initial
                } else {
                    ContainerHistoryBaseline::EarliestAvailable
                };
                data.push(entry(
                    &steps[index],
                    config_changes(None, Some(after)),
                    Some(baseline),
                ));
            }
            continue;
        }
        let before = configs.get(index + 1).cloned().flatten();
        let changes = config_changes(before.as_ref(), after.as_ref());
        if changes.is_empty() {
            continue;
        }
        data.push(entry(&steps[index], changes, None));
    }
    let next_cursor = if more && steps.len() > page_limit {
        Some(encode_cursor(steps[page_limit].id))
    } else {
        None
    };
    (data, next_cursor)
}

async fn ancestry_window(
    tx: &impl ConnectionTrait,
    project_id: Uuid,
    organization_id: Uuid,
    start: Uuid,
    page_limit: usize,
) -> Result<(Vec<project_timeline::Model>, bool), AppError> {
    let scan_limit = page_limit + 1;
    let steps =
        project_timeline::Model::find_by_statement(sea_orm::Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Postgres,
            "WITH RECURSIVE ancestry AS (
            SELECT t.*, 1 AS depth FROM project_timeline t
            WHERE t.id = $1 AND t.project_id = $2 AND t.organization_id = $3
            UNION ALL
            SELECT t.*, a.depth + 1 FROM project_timeline t
            JOIN ancestry a ON t.id = a.parent_timeline_id
            WHERE t.project_id = $2 AND t.organization_id = $3 AND a.depth < $4
        ) SELECT * FROM ancestry ORDER BY depth",
            [
                start.into(),
                project_id.into(),
                organization_id.into(),
                (scan_limit as i32).into(),
            ],
        ))
        .all(tx)
        .await?;
    let mut visited = HashSet::new();
    for revision in &steps {
        if !visited.insert(revision.id) {
            return Err(AppError::Internal(
                "Revision ancestry contains a cycle".into(),
            ));
        }
    }
    let more = steps.len() > page_limit;
    Ok((steps, more))
}

#[derive(FromQueryResult)]
struct ManifestContainerRow {
    manifest_id: Uuid,
    schema_version: i32,
    container: Option<Value>,
}

async fn load_container_configs(
    tx: &impl ConnectionTrait,
    manifest_ids: Vec<Uuid>,
    container_id: Uuid,
) -> Result<HashMap<Uuid, Option<ContainerConfig>>, AppError> {
    if manifest_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let key = container_id.to_string();
    project_revision_manifest::Entity::find()
        .select_only()
        .expr_as(
            Expr::col(project_revision_manifest::Column::Id),
            "manifest_id",
        )
        .column(project_revision_manifest::Column::SchemaVersion)
        .expr_as(
            Expr::cust_with_values("configuration->'containers'->?", [&key]),
            "container",
        )
        .filter(project_revision_manifest::Column::Id.is_in(manifest_ids))
        .into_model::<ManifestContainerRow>()
        .all(tx)
        .await?
        .into_iter()
        .map(|row| {
            if row.schema_version != lib::manifest::MANIFEST_SCHEMA_VERSION {
                return Err(AppError::Conflict(
                    "Unsupported revision manifest schema version".into(),
                ));
            }
            let config = row
                .container
                .filter(|value| !value.is_null())
                .map(|value| {
                    serde_json::from_value(value).map_err(|_| {
                        AppError::Conflict("Invalid revision manifest configuration".into())
                    })
                })
                .transpose()?;
            Ok((row.manifest_id, config))
        })
        .collect()
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/containers/{container_id}/history",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("container_id" = Uuid, Path, description = "Container ID"),
        ("environment_id" = Uuid, Query, description = "Environment in the container's project"),
        ("timeline_id" = Uuid, Query, description = "Selected revision whose ancestry to show"),
        ("limit" = Option<u64>, Query, description = "History entries per page (default 10, max 50)"),
        ("cursor" = Option<String>, Query, description = "Continuation cursor from a previous page"),
    ),
    responses(
        (status = 200, description = "Container configuration changes, newest first", body = ContainerHistoryPage),
        (status = 404, description = "Container, environment, revision, or manifest not found"),
    ),
    tag = "containers",
)]
pub async fn get_container_history(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, container_id)): Path<(Uuid, Uuid)>,
    axum::extract::Query(query): axum::extract::Query<ContainerHistoryQuery>,
) -> Result<Json<ContainerHistoryPage>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let c = container::Entity::find()
        .filter(container::Column::Id.eq(container_id))
        .filter(container::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Container not found".into()))?;
    verify_project_in_org(tx, c.project_id, organization_id).await?;
    get_environment(
        tx,
        query.environment_id,
        organization_id,
        Some(c.project_id),
    )
    .await?;

    let start = match query.cursor.as_deref() {
        Some(cursor) => decode_cursor(cursor)?,
        None => query.timeline_id,
    };

    let page_limit = history_limit(query.limit);
    let (steps, more) =
        ancestry_window(tx, c.project_id, organization_id, start, page_limit).await?;
    let selected = steps
        .first()
        .ok_or_else(|| AppError::NotFound("Timeline revision is not in this project".into()))?;

    let manifest_ids: Vec<Uuid> = steps.iter().map(|step| step.manifest_id).collect();
    let configs_by_manifest = load_container_configs(tx, manifest_ids, container_id).await?;

    let config_at = |revision: &project_timeline::Model| -> Option<ContainerConfig> {
        configs_by_manifest
            .get(&revision.manifest_id)
            .cloned()
            .flatten()
    };
    if query.cursor.is_none() && config_at(selected).is_none() {
        return Err(AppError::NotFound(
            "Container not present in timeline revision".into(),
        ));
    }
    let configs: Vec<Option<ContainerConfig>> = steps.iter().map(config_at).collect();

    let (data, next_cursor) = build_history(&steps, &configs, more, page_limit);
    scoped.commit().await?;
    Ok(Json(ContainerHistoryPage { data, next_cursor }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn revision(id: u128, parent: Option<u128>, timeline: i32) -> project_timeline::Model {
        project_timeline::Model {
            id: Uuid::from_u128(id),
            project_id: Uuid::nil(),
            organization_id: Uuid::nil(),
            environment_id: Some(Uuid::from_u128(100)),
            timeline,
            name: None,
            parent_timeline_id: parent.map(Uuid::from_u128),
            manifest_id: Uuid::from_u128(id),
            created_at: Utc::now().fixed_offset(),
        }
    }

    fn config(replicas: i32, resolved: &str, env: Value) -> ContainerConfig {
        ContainerConfig {
            name: "app".into(),
            region_id: Uuid::nil(),
            image: "app:latest".into(),
            resolved_image: resolved.into(),
            external_registry_id: None,
            replica_count: replicas,
            port: Some(80),
            public: false,
            cpu: Some("0.5".into()),
            memory: Some("128Mi".into()),
            health_check: None,
            env: Some(env),
        }
    }

    #[test]
    fn history_limit_defaults_and_clamps() {
        assert_eq!(history_limit(None), HISTORY_DEFAULT_LIMIT);
        assert_eq!(history_limit(Some(1)), 1);
        assert_eq!(history_limit(Some(100)), HISTORY_MAX_LIMIT);
    }

    #[test]
    fn history_reports_only_changes_and_masks_env() {
        let steps = vec![
            revision(15, Some(14), 5),
            revision(14, Some(13), 4),
            revision(13, Some(11), 3),
            revision(11, None, 1),
        ];
        let configs = vec![
            Some(config(3, "app@sha256:3", json!({"TOKEN": "new-secret"}))),
            Some(config(3, "app@sha256:3", json!({"TOKEN": "new-secret"}))),
            Some(config(
                1,
                "app@sha256:1",
                json!({"TOKEN": "old-secret", "REMOVED": "removed-secret"}),
            )),
            Some(config(
                1,
                "app@sha256:1",
                json!({"TOKEN": "old-secret", "REMOVED": "removed-secret"}),
            )),
        ];
        let (data, next_cursor) = build_history(&steps, &configs, false, HISTORY_DEFAULT_LIMIT);

        assert_eq!(next_cursor, None);
        assert_eq!(data.len(), 2);

        assert_eq!(data[0].revision_number, 4);
        assert!(data[0].baseline.is_none());
        let fields: Vec<&str> = data[0].changes.iter().map(|c| c.field.as_str()).collect();
        assert!(fields.contains(&"resolved_image"));
        assert!(fields.contains(&"replica_count"));
        assert!(fields.contains(&"env.TOKEN"));
        assert!(matches!(
            data[0]
                .changes
                .iter()
                .find(|c| c.field == "env.TOKEN")
                .unwrap()
                .change_type,
            ContainerChangeType::Changed
        ));

        assert_eq!(data[1].revision_number, 1);
        assert!(matches!(
            data[1].baseline,
            Some(ContainerHistoryBaseline::Initial)
        ));
        assert!(!serde_json::to_string(&data).unwrap().contains("-secret"));
    }

    #[test]
    fn removed_container_yields_removals_and_truncation_keeps_cursor() {
        let mut steps = Vec::new();
        let mut configs = Vec::new();
        for index in 0..(HISTORY_DEFAULT_LIMIT + 1) as u128 {
            let id = 1000 - index;
            let parent = (index > 0).then_some(id + 1);
            steps.push(revision(
                id,
                parent,
                ((HISTORY_DEFAULT_LIMIT + 1) as i32) - index as i32,
            ));
            configs.push(Some(config(1, "app@sha256:1", json!({}))));
        }
        let (data, next_cursor) = build_history(&steps, &configs, true, HISTORY_DEFAULT_LIMIT);
        assert!(data.is_empty());
        assert_eq!(
            next_cursor,
            Some(encode_cursor(steps[HISTORY_DEFAULT_LIMIT].id))
        );

        let removed = vec![revision(2, Some(1), 2), revision(1, None, 1)];
        let configs = vec![None, Some(config(1, "app@sha256:1", json!({})))];
        let (data, _) = build_history(&removed, &configs, false, HISTORY_DEFAULT_LIMIT);
        assert_eq!(data.len(), 2);
        assert_eq!(data[0].revision_number, 2);
        assert!(
            data[0]
                .changes
                .iter()
                .all(|c| matches!(c.change_type, ContainerChangeType::Removed))
        );
        assert!(matches!(
            data[1].baseline,
            Some(ContainerHistoryBaseline::Initial)
        ));
    }
}

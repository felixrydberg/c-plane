use super::*;
use crate::handlers::databases::verify_project_in_org;
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashSet};

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
    pub id: Uuid,
    pub version: i32,
    pub created_at: String,
    pub baseline: Option<ContainerHistoryBaseline>,
    pub changes: Vec<ContainerHistoryChange>,
}

// Version numbers are global to a container. Only revision parents establish ancestry.
fn history_version_ids(
    revisions: &[project_timeline::Model],
    timeline_id: Uuid,
    container_id: Uuid,
) -> Result<Vec<Uuid>, AppError> {
    let revisions: HashMap<_, _> = revisions.iter().map(|r| (r.id, r)).collect();
    let mut next = Some(timeline_id);
    let mut visited = HashSet::new();
    let mut ids = Vec::new();
    let mut previous_pin = None;
    while let Some(id) = next {
        if !visited.insert(id) {
            return Err(AppError::Internal(
                "Revision ancestry contains a cycle".into(),
            ));
        }
        let revision = revisions
            .get(&id)
            .ok_or_else(|| AppError::NotFound("Timeline revision is not in this project".into()))?;
        let pin = TimelinePins::from_json_value(&revision.pins)
            .container
            .get(&container_id)
            .copied();
        if id == timeline_id && pin.is_none() {
            return Err(AppError::NotFound(
                "Container not present in timeline revision".into(),
            ));
        }
        if pin != previous_pin {
            if let Some(version_id) = pin {
                ids.push(version_id);
            }
        }
        previous_pin = pin;
        next = revision.parent_timeline_id;
    }
    Ok(ids)
}

fn config(version: &container_version::Model) -> Value {
    json!({
        "image": version.image,
        "resolved_image": version.resolved_image,
        "external_registry_id": version.external_registry_id,
        "replica_count": version.replica_count,
        "port": version.port,
        "public": version.public,
        "resources": version.resources,
        "health_check": version.health_check,
    })
}

fn version_changes(
    before: Option<&container_version::Model>,
    after: &container_version::Model,
) -> Vec<ContainerHistoryChange> {
    let old = before.map(config).unwrap_or(Value::Null);
    let new = config(after);
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
        "image",
        "resolved_image",
        "external_registry_id",
        "replica_count",
        "port",
        "public",
        "resources",
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
        .and_then(|v| v.env.as_ref())
        .and_then(Value::as_object);
    let new_env = after.env.as_ref().and_then(Value::as_object);
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

fn history_entries(
    ids: &[Uuid],
    versions: &[container_version::Model],
) -> Result<Vec<ContainerHistoryEntry>, AppError> {
    let versions: HashMap<_, _> = versions.iter().map(|v| (v.id, v)).collect();
    let get = |id: &Uuid| {
        versions
            .get(id)
            .copied()
            .ok_or_else(|| AppError::NotFound("Container version not found".into()))
    };
    ids.iter()
        .enumerate()
        .map(|(index, id)| {
            let version = get(id)?;
            let previous = ids.get(index + 1).map(get).transpose()?;
            Ok(ContainerHistoryEntry {
                id: version.id,
                version: version.version,
                created_at: version.created_at.to_string(),
                baseline: previous.is_none().then_some(if version.version == 1 {
                    ContainerHistoryBaseline::Initial
                } else {
                    ContainerHistoryBaseline::EarliestAvailable
                }),
                changes: version_changes(previous, version),
            })
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
    ),
    responses(
        (status = 200, description = "Container version ancestry, newest first", body = Vec<ContainerHistoryEntry>),
        (status = 404, description = "Container, environment, revision, or version not found"),
    ),
    tag = "containers",
)]
pub async fn get_container_history(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, container_id)): Path<(Uuid, Uuid)>,
    axum::extract::Query(query): axum::extract::Query<UpdateContainerQuery>,
) -> Result<Json<Vec<ContainerHistoryEntry>>, AppError> {
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
    let revisions = project_timeline::Entity::find()
        .filter(project_timeline::Column::ProjectId.eq(c.project_id))
        .filter(project_timeline::Column::OrganizationId.eq(organization_id))
        .all(tx)
        .await?;
    let ids = history_version_ids(&revisions, query.timeline_id, container_id)?;
    let versions = container_version::Entity::find()
        .filter(container_version::Column::ContainerId.eq(container_id))
        .filter(container_version::Column::OrganizationId.eq(organization_id))
        .filter(container_version::Column::Id.is_in(ids.clone()))
        .all(tx)
        .await?;
    let history = history_entries(&ids, &versions)?;
    scoped.commit().await?;
    Ok(Json(history))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_follows_ancestry_and_compares_saved_values_without_exposing_env() {
        let container_id = Uuid::from_u128(100);
        let version = |number: u128, replicas: i32, env: Value| -> container_version::Model {
            serde_json::from_value(json!({
                "id": Uuid::from_u128(number), "container_id": container_id,
                "organization_id": Uuid::nil(), "version": number,
                "image": "app:latest", "resolved_image": format!("app@sha256:{replicas}"),
                "public": false, "replica_count": replicas, "port": 80,
                "env": env, "resources": {"cpu": "0.5", "memory": "128Mi"},
                "external_registry_id": null, "health_check": null,
                "created_at": "2026-09-05T12:00:00Z"
            }))
            .unwrap()
        };
        let versions = vec![
            version(
                1,
                1,
                json!({"TOKEN": "old-secret", "REMOVED": "removed-secret"}),
            ),
            version(2, 2, json!({"TOKEN": "abandoned-secret"})),
            version(
                3,
                3,
                json!({"TOKEN": "new-secret", "ADDED": "added-secret"}),
            ),
            version(
                4,
                3,
                json!({"ADDED": "added-secret", "TOKEN": "new-secret"}),
            ),
        ];
        let revision = |id: u128, parent: Option<u128>, pin: u128| project_timeline::Model {
            id: Uuid::from_u128(id),
            parent_timeline_id: parent.map(Uuid::from_u128),
            project_id: Uuid::nil(),
            organization_id: Uuid::nil(),
            environment_id: Some(Uuid::from_u128(id)),
            timeline: id as i32,
            name: None,
            pins: json!({"container": {container_id.to_string(): Uuid::from_u128(pin)}}),
            created_at: Utc::now().fixed_offset(),
        };
        let revisions = vec![
            revision(11, None, 1),
            revision(12, Some(11), 2),
            revision(13, Some(11), 3),
            revision(14, Some(13), 3),
            revision(15, Some(14), 4),
        ];
        let ids = history_version_ids(&revisions, Uuid::from_u128(15), container_id).unwrap();
        assert_eq!(
            ids,
            vec![Uuid::from_u128(4), Uuid::from_u128(3), Uuid::from_u128(1)]
        );
        let entries = history_entries(&ids, &versions).unwrap();
        assert!(entries[0].changes.is_empty());
        let changed = &entries[1].changes;
        assert_eq!(
            changed.iter().map(|c| c.field.as_str()).collect::<Vec<_>>(),
            vec![
                "resolved_image",
                "replica_count",
                "env.ADDED",
                "env.REMOVED",
                "env.TOKEN"
            ]
        );
        assert_eq!(changed[1].before, json!(1));
        assert_eq!(changed[1].after, json!(3));
        assert!(matches!(changed[2].change_type, ContainerChangeType::Added));
        assert!(matches!(
            changed[3].change_type,
            ContainerChangeType::Removed
        ));
        assert!(matches!(
            changed[4].change_type,
            ContainerChangeType::Changed
        ));
        assert_eq!(changed[4].before, json!("••••••••"));
        assert_eq!(changed[4].after, json!("••••••••"));
        assert!(!serde_json::to_string(&entries).unwrap().contains("-secret"));
        assert!(matches!(
            entries[2].baseline,
            Some(ContainerHistoryBaseline::Initial)
        ));

        let historical =
            history_version_ids(&revisions, Uuid::from_u128(11), container_id).unwrap();
        assert_eq!(historical, vec![Uuid::from_u128(1)]);
        let truncated = history_entries(&[Uuid::from_u128(3)], &versions).unwrap();
        assert!(matches!(
            truncated[0].baseline,
            Some(ContainerHistoryBaseline::EarliestAvailable)
        ));
        assert!(history_version_ids(&revisions, Uuid::from_u128(15), Uuid::nil()).is_err());
    }
}

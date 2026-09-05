use axum::{
    Json,
    extract::{Path, Query},
};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseBackend, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Statement,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;
use uuid::Uuid;

use super::databases::{verify_org_access, verify_project_in_org};
use crate::{
    errors::AppError, middleware::auth::AuthContext, models::entities::event, state::get_app_state,
};

#[derive(Deserialize, ToSchema)]
pub struct ListEventsQuery {
    pub project_id: Option<Uuid>,
    pub event_type_prefix: Option<String>,
    pub environment_id: Option<Uuid>,
    pub target_id: Option<Uuid>,
    pub limit: Option<u64>,
}

#[derive(Serialize, ToSchema)]
pub struct EventResponse {
    pub id: Uuid,
    pub action: String,
    pub summary: String,
    pub actor_id: Option<Uuid>,
    pub actor_name: Option<String>,
    pub created_at: String,
}

fn event_limit(limit: Option<u64>) -> u64 {
    limit.unwrap_or(10).clamp(1, 50)
}

fn event_action(event_type: &str) -> &str {
    // "container:created" → "created", "database:linked" → "linked"
    event_type
        .split_once(':')
        .map(|(_, action)| action)
        .unwrap_or(event_type)
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/events",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Option<Uuid>, Query, description = "Optional project ID filter"),
        ("event_type_prefix" = Option<String>, Query, description = "Event type prefix filter (e.g. 'container' matches 'container:created')"),
        ("environment_id" = Option<Uuid>, Query, description = "Environment filter (matched against payload->>'environment_id')"),
        ("target_id" = Option<Uuid>, Query, description = "Resource ID filter (matched against payload->>'target_id')"),
        ("limit" = Option<u64>, Query, description = "Maximum events (default 10, max 50)"),
    ),
    responses((status = 200, body = Vec<EventResponse>)),
    tag = "events",
)]
pub async fn list_events(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Query(query): Query<ListEventsQuery>,
) -> Result<Json<Vec<EventResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    if let Some(project_id) = query.project_id {
        verify_project_in_org(tx, project_id, organization_id).await?;
    }

    let mut select =
        event::Entity::find().filter(event::Column::OrganizationId.eq(organization_id));

    if let Some(project_id) = query.project_id {
        select = select.filter(event::Column::ProjectId.eq(project_id));
    }

    if let Some(ref prefix) = query.event_type_prefix {
        select = select.filter(event::Column::EventType.like(format!("{prefix}:%")));
    }
    if let Some(environment_id) = query.environment_id {
        select = select.filter(sea_orm::sea_query::Expr::cust(format!(
            "payload->>'environment_id' = '{environment_id}'"
        )));
    }
    if let Some(target_id) = query.target_id {
        select = select.filter(sea_orm::sea_query::Expr::cust(format!(
            "payload->>'target_id' = '{target_id}'"
        )));
    }

    let events = select
        .order_by_desc(event::Column::CreatedAt)
        .limit(event_limit(query.limit))
        .all(tx)
        .await?;
    scoped.commit().await?;

    let actor_ids: Vec<_> = events
        .iter()
        .filter_map(|event| event.actor_id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect();
    let actor_names = if actor_ids.is_empty() {
        HashMap::new()
    } else {
        let placeholders = (1..=actor_ids.len())
            .map(|index| format!("${index}"))
            .collect::<Vec<_>>()
            .join(", ");
        get_app_state()
            .identity_db
            .connection()
            .query_all(Statement::from_sql_and_values(
                DatabaseBackend::Postgres,
                format!(
                    r#"SELECT id, name, 'user' AS actor_kind FROM "user" WHERE id IN ({placeholders})
                       UNION ALL
                       SELECT id, name, 'api_key' AS actor_kind FROM api_keys WHERE id IN ({placeholders})"#
                ),
                actor_ids
                    .into_iter()
                    .map(sea_orm::Value::from)
                    .collect::<Vec<_>>(),
            ))
            .await
            .map_err(|error| AppError::Internal(format!("Failed to resolve event actors: {error}")))?
            .into_iter()
            .map(|row| {
                let id = row
                    .try_get::<Uuid>("", "id")
                    .map_err(|error| AppError::Internal(format!("Invalid event actor ID: {error}")))?;
                let name = row.try_get::<String>("", "name").map_err(|error| {
                    AppError::Internal(format!("Invalid event actor name: {error}"))
                })?;
                let actor_kind = row.try_get::<String>("", "actor_kind").map_err(|error| {
                    AppError::Internal(format!("Invalid event actor type: {error}"))
                })?;
                let label = if actor_kind == "api_key" {
                    format!("API key: {name}")
                } else {
                    name
                };
                Ok::<_, AppError>((id, label))
            })
            .collect::<Result<HashMap<_, _>, _>>()?
    };

    Ok(Json(
        events
            .into_iter()
            .map(|event| EventResponse {
                id: event.id,
                action: event_action(&event.event_type).into(),
                summary: event
                    .payload
                    .get("summary")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("Resource updated")
                    .into(),
                actor_id: event.actor_id,
                actor_name: event.actor_id.and_then(|id| actor_names.get(&id).cloned()),
                created_at: event.created_at.to_rfc3339(),
            })
            .collect(),
    ))
}

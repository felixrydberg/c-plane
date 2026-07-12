use axum::{
    Json,
    extract::{Path, Query},
};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use super::databases::{verify_org_access, verify_project_in_org};
use crate::{errors::AppError, middleware::auth::AuthContext, models::entities::event};

#[derive(Deserialize, ToSchema)]
pub struct ListEventsQuery {
    pub project_id: Uuid,
    pub event_type_prefix: Option<String>,
    pub branch_id: Option<Uuid>,
    pub target_id: Option<Uuid>,
    pub limit: Option<u64>,
}

#[derive(Serialize, ToSchema)]
pub struct EventResponse {
    pub id: Uuid,
    pub action: String,
    pub summary: String,
    pub actor_id: Option<Uuid>,
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
        ("project_id" = Uuid, Query, description = "Project ID"),
        ("event_type_prefix" = Option<String>, Query, description = "Event type prefix filter (e.g. 'container' matches 'container:created')"),
        ("branch_id" = Option<Uuid>, Query, description = "Branch filter (matched against payload->>'branch_id')"),
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
    verify_project_in_org(tx, query.project_id, organization_id).await?;

    let mut select = event::Entity::find()
        .filter(event::Column::OrganizationId.eq(organization_id))
        .filter(event::Column::ProjectId.eq(query.project_id));

    if let Some(ref prefix) = query.event_type_prefix {
        select = select.filter(event::Column::EventType.like(format!("{prefix}:%")));
    }
    if let Some(branch_id) = query.branch_id {
        select = select.filter(sea_orm::sea_query::Expr::cust(format!(
            "payload->>'branch_id' = '{branch_id}'"
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
                created_at: event.created_at.to_string(),
            })
            .collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::{event_action, event_limit};

    #[test]
    fn event_limit_stays_in_api_bounds() {
        assert_eq!(event_limit(None), 10);
        assert_eq!(event_limit(Some(0)), 1);
        assert_eq!(event_limit(Some(100)), 50);
    }

    #[test]
    fn event_action_strips_type_prefix() {
        assert_eq!(event_action("container:created"), "created");
        assert_eq!(event_action("container:updated"), "updated");
        assert_eq!(event_action("database:linked"), "linked");
        assert_eq!(event_action("organization:member_added"), "member_added");
    }
}

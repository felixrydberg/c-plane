use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entities::{project, project_environment, project_timeline},
    error::AppError,
    services::revisions,
    tenant::TenantDatabase,
};

#[derive(Clone, Serialize)]
pub struct Timeline {
    pub id: Uuid,
    pub environment_id: Option<Uuid>,
    pub timeline: i32,
    pub name: Option<String>,
    pub parent_timeline_id: Option<Uuid>,
    pub created_at: String,
}
#[derive(Clone, Serialize)]
pub struct GraphNode {
    pub id: Uuid,
    pub timeline: i32,
    pub parent_id: Option<Uuid>,
    pub lane: usize,
    pub child_ids: Vec<Uuid>,
}
#[derive(Default)]
pub struct TimelinePage {
    pub data: Vec<Timeline>,
    pub next_cursor: Option<String>,
    pub page: Option<u64>,
    pub total_pages: Option<u64>,
    pub has_newer: Option<bool>,
    pub has_older: Option<bool>,
    pub graph_nodes: Option<Vec<GraphNode>>,
}
pub struct ListInput {
    pub environment_id: Option<Uuid>,
    pub limit: Option<u64>,
    pub cursor: Option<String>,
    pub anchor_revision_id: Option<Uuid>,
    pub graph: Option<bool>,
    pub page: Option<u64>,
}
pub struct ContainerPin {
    pub container_id: Uuid,
    pub container_name: String,
    pub image: String,
    pub external_registry_id: Option<Uuid>,
}
pub struct ResolvedTimeline {
    pub timeline: Timeline,
    pub containers: Vec<ContainerPin>,
}
#[derive(Serialize, Deserialize)]
struct Cursor {
    project_id: Uuid,
    environment_id: Option<Uuid>,
    timeline: i32,
}

fn access(db: &TenantDatabase, org: Uuid) -> Result<(), AppError> {
    if db.context.allowed_organizations.contains(&org) {
        Ok(())
    } else {
        Err(AppError::Forbidden(
            "You do not have access to this organization".into(),
        ))
    }
}
fn encode(project_id: Uuid, environment_id: Option<Uuid>, timeline: i32) -> String {
    URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(&Cursor {
            project_id,
            environment_id,
            timeline,
        })
        .unwrap_or_default(),
    )
}
fn decode(value: &str, project_id: Uuid, environment_id: Option<Uuid>) -> Result<Cursor, AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| AppError::BadRequest("Invalid timeline cursor".into()))?;
    let cursor: Cursor = serde_json::from_slice(&bytes)
        .map_err(|_| AppError::BadRequest("Invalid timeline cursor".into()))?;
    if cursor.project_id != project_id || cursor.environment_id != environment_id {
        return Err(AppError::BadRequest(
            "Timeline cursor does not match this request".into(),
        ));
    }
    Ok(cursor)
}
fn timeline(row: project_timeline::Model) -> Timeline {
    Timeline {
        id: row.id,
        environment_id: row.environment_id,
        timeline: row.timeline,
        name: row.name,
        parent_timeline_id: row.parent_timeline_id,
        created_at: row.created_at.to_string(),
    }
}
fn layout(rows: Vec<(Uuid, i32, Option<Uuid>)>, head: Option<Uuid>) -> Vec<GraphNode> {
    let mut nodes: Vec<_> = rows
        .into_iter()
        .map(|(id, timeline, parent_id)| GraphNode {
            id,
            timeline,
            parent_id,
            lane: 0,
            child_ids: Vec::new(),
        })
        .collect();
    let positions = nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id, i))
        .collect::<std::collections::HashMap<_, _>>();
    let mut main = std::collections::HashSet::new();
    let mut current = head;
    while let Some(i) = current.and_then(|id| positions.get(&id).copied()) {
        if !main.insert(nodes[i].id) {
            break;
        }
        current = nodes[i].parent_id;
    }
    let mut continued = std::collections::HashSet::new();
    for node in &nodes {
        if main.contains(&node.id) {
            if let Some(parent) = node.parent_id {
                continued.insert(parent);
            }
        }
    }
    let mut next = usize::from(!main.is_empty());
    for i in (0..nodes.len()).rev() {
        let parent = nodes[i]
            .parent_id
            .and_then(|id| positions.get(&id).copied());
        if !main.contains(&nodes[i].id) {
            nodes[i].lane = match parent {
                Some(p) if continued.insert(nodes[p].id) => nodes[p].lane,
                _ => {
                    let lane = next;
                    next += 1;
                    lane
                }
            };
        }
        if let Some(p) = parent {
            let child_id = nodes[i].id;
            nodes[p].child_ids.push(child_id);
        }
    }
    nodes
}

pub async fn list(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
    input: ListInput,
) -> Result<TimelinePage, AppError> {
    access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let project = project::Entity::find_by_id(project_id)
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Project not found".into()))?;
    let limit = input.limit.unwrap_or(50).clamp(1, 100) as usize;
    if input.graph.unwrap_or(false) {
        if input.cursor.is_some() || input.environment_id.is_some() {
            return Err(AppError::BadRequest(
                "Graph pages do not support cursors or environment filters".into(),
            ));
        }
        let rows = project_timeline::Entity::find()
            .select_only()
            .columns([
                project_timeline::Column::Id,
                project_timeline::Column::Timeline,
                project_timeline::Column::ParentTimelineId,
            ])
            .filter(project_timeline::Column::ProjectId.eq(project_id))
            .filter(project_timeline::Column::OrganizationId.eq(organization_id))
            .order_by_desc(project_timeline::Column::Timeline)
            .into_tuple::<(Uuid, i32, Option<Uuid>)>()
            .all(tx)
            .await?;
        let head = match project.default_environment_id {
            Some(id) => project_environment::Entity::find_by_id(id)
                .one(tx)
                .await?
                .map(|e| e.draft_timeline),
            None => None,
        };
        let book = layout(rows, head);
        let requested = input.page.unwrap_or(0);
        let page = match input.anchor_revision_id {
            Some(id) => {
                book.iter().position(|n| n.id == id).ok_or_else(|| {
                    AppError::NotFound("Anchor revision not found in this book".into())
                })? as u64
                    / limit as u64
            }
            None => requested.min((book.len() as u64).div_ceil(limit as u64).max(1) - 1),
        };
        let offset = page as usize * limit;
        let ids = book
            .iter()
            .skip(offset)
            .take(limit)
            .map(|n| n.id)
            .collect::<Vec<_>>();
        let revisions = if ids.is_empty() {
            Vec::new()
        } else {
            project_timeline::Entity::find()
                .filter(project_timeline::Column::ProjectId.eq(project_id))
                .filter(project_timeline::Column::OrganizationId.eq(organization_id))
                .filter(project_timeline::Column::Id.is_in(ids))
                .order_by_desc(project_timeline::Column::Timeline)
                .all(tx)
                .await?
        };
        scoped.commit().await?;
        return Ok(TimelinePage {
            data: revisions.into_iter().map(timeline).collect(),
            page: Some(page),
            total_pages: Some((book.len() as u64).div_ceil(limit as u64).max(1)),
            has_newer: Some(page > 0),
            has_older: Some(offset + limit < book.len()),
            graph_nodes: Some(book),
            ..Default::default()
        });
    }
    if input.cursor.is_some() && input.anchor_revision_id.is_some() {
        return Err(AppError::BadRequest(
            "Use either a cursor or an anchor revision".into(),
        ));
    }
    let cursor = input
        .cursor
        .as_deref()
        .map(|v| decode(v, project_id, input.environment_id))
        .transpose()?;
    let mut select =
        project_timeline::Entity::find().filter(project_timeline::Column::ProjectId.eq(project_id));
    if let Some(anchor) = input.anchor_revision_id {
        let row = project_timeline::Entity::find_by_id(anchor)
            .filter(project_timeline::Column::ProjectId.eq(project_id))
            .one(tx)
            .await?
            .ok_or_else(|| AppError::NotFound("Anchor revision not found".into()))?;
        select = select.filter(project_timeline::Column::Timeline.lte(row.timeline));
    }
    if let Some(environment) = input.environment_id {
        select = select.filter(project_timeline::Column::EnvironmentId.eq(environment));
    }
    if let Some(cursor) = cursor {
        select = select.filter(project_timeline::Column::Timeline.lt(cursor.timeline));
    }
    let mut rows = select
        .order_by_desc(project_timeline::Column::Timeline)
        .limit(limit as u64 + 1)
        .all(tx)
        .await?;
    let more = rows.len() > limit;
    rows.truncate(limit);
    let next_cursor = if more {
        rows.last()
            .map(|row| encode(project_id, input.environment_id, row.timeline))
    } else {
        None
    };
    scoped.commit().await?;
    Ok(TimelinePage {
        data: rows.into_iter().map(timeline).collect(),
        next_cursor,
        ..Default::default()
    })
}

pub async fn get(
    tenant_db: &TenantDatabase,
    organization_id: Uuid,
    project_id: Uuid,
    timeline_id: Uuid,
) -> Result<ResolvedTimeline, AppError> {
    access(tenant_db, organization_id)?;
    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();
    let row = project_timeline::Entity::find_by_id(timeline_id)
        .filter(project_timeline::Column::ProjectId.eq(project_id))
        .filter(project_timeline::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Timeline not found".into()))?;
    let manifest = revisions::load_manifest(tx, row.manifest_id).await?;
    let mut containers = manifest
        .containers
        .iter()
        .map(|(id, c)| ContainerPin {
            container_id: *id,
            container_name: c.name.clone(),
            image: c.resolved_image.clone(),
            external_registry_id: c.external_registry_id,
        })
        .collect::<Vec<_>>();
    containers.sort_by(|a, b| {
        a.container_name
            .cmp(&b.container_name)
            .then(a.container_id.cmp(&b.container_id))
    });
    scoped.commit().await?;
    Ok(ResolvedTimeline {
        timeline: timeline(row),
        containers,
    })
}

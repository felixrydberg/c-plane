use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, Set};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::entities::{project_branch, project_timeline};
use crate::models::pins::TimelinePins;
use crate::services::agent;

pub async fn create_revision(
    tx: &DatabaseTransaction,
    branch: &project_branch::Model,
    updated_pins: &TimelinePins,
    name: Option<String>,
    update_branch: bool,
) -> Result<project_timeline::Model, AppError> {
    use project_timeline::{ActiveModel, Entity};

    let head = Entity::find_by_id(branch.timeline)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Branch timeline not found".into()))?;

    let new_entry = ActiveModel {
        id: Set(Uuid::new_v4()),
        project_id: Set(branch.project_id),
        branch_id: Set(Some(branch.id)),
        organization_id: Set(branch.organization_id),
        timeline: Set(head.timeline + 1),
        name: Set(name),
        parent_timeline_id: Set(Some(head.id)),
        pins: Set(updated_pins.to_json_value()),
        created_at: Set(Utc::now().fixed_offset()),
    };

    let inserted = new_entry.insert(tx).await?;

    if update_branch {
        let mut branch_active: project_branch::ActiveModel = branch.clone().into();
        branch_active.timeline = Set(inserted.id);
        branch_active.updated_at = Set(Utc::now().fixed_offset());
        branch_active.update(tx).await?;

        agent::emit_project(
            branch.project_id,
            branch.organization_id,
            branch.id,
            inserted.id,
        )
        .await?;
    }

    Ok(inserted)
}

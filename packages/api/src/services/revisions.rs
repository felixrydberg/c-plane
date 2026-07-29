use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, Set};
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::entities::{project_environment, project_timeline};
use crate::models::pins::TimelinePins;

pub async fn create_revision(
    tx: &DatabaseTransaction,
    environment: &project_environment::Model,
    updated_pins: &TimelinePins,
    name: Option<String>,
    deploy: bool,
) -> Result<project_timeline::Model, AppError> {
    use project_timeline::{ActiveModel, Entity};

    let head = Entity::find_by_id(environment.draft_timeline)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Environment timeline not found".into()))?;

    let new_entry = ActiveModel {
        id: Set(Uuid::new_v4()),
        project_id: Set(environment.project_id),
        environment_id: Set(Some(environment.id)),
        organization_id: Set(environment.organization_id),
        timeline: Set(head.timeline + 1),
        name: Set(name),
        parent_timeline_id: Set(Some(head.id)),
        pins: Set(updated_pins.to_json_value()),
        created_at: Set(Utc::now().fixed_offset()),
    };

    let inserted = new_entry.insert(tx).await?;

    let mut environment_active: project_environment::ActiveModel = environment.clone().into();
    environment_active.draft_timeline = Set(inserted.id);
    if deploy {
        environment_active.deployed_timeline = Set(inserted.id);
    }
    environment_active.updated_at = Set(Utc::now().fixed_offset());
    environment_active.update(tx).await?;

    Ok(inserted)
}

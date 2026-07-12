use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set};
use uuid::Uuid;

use crate::{errors::AppError, models::entities::event};

pub async fn record(
    tx: &DatabaseTransaction,
    organization_id: Uuid,
    project_id: Uuid,
    event_type: &str,
    payload: serde_json::Value,
    actor_id: Uuid,
) -> Result<(), AppError> {
    event::ActiveModel {
        id: Set(Uuid::new_v4()),
        organization_id: Set(organization_id),
        event_type: Set(event_type.into()),
        payload: Set(payload),
        system: Set(false),
        project_id: Set(Some(project_id)),
        actor_id: Set(Some(actor_id)),
        created_at: Set(Utc::now().naive_utc()),
    }
    .insert(tx)
    .await?;
    Ok(())
}

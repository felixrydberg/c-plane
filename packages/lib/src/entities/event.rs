use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "event")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub organization_id: Uuid,
    #[sea_orm(column_name = "type")]
    pub event_type: String,
    pub payload: serde_json::Value,
    pub system: bool,
    pub project_id: Option<Uuid>,
    pub actor_id: Option<Uuid>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

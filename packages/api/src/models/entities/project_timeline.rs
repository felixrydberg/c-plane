use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "project_timeline")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub project_id: Uuid,
    pub branch_id: Option<Uuid>,
    pub organization_id: Uuid,
    pub timeline: i32,
    pub name: Option<String>,
    pub parent_timeline_id: Option<Uuid>,
    pub pins: serde_json::Value,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::project::Entity",
        from = "Column::ProjectId",
        to = "super::project::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Project,
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::ParentTimelineId",
        to = "Column::Id"
    )]
    Parent,
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

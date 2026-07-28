use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "project_environment")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub project_id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub is_preview: bool,
    pub draft_timeline: Uuid,
    pub deployed_timeline: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
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
        belongs_to = "super::project_timeline::Entity",
        from = "Column::DraftTimeline",
        to = "super::project_timeline::Column::Id"
    )]
    DraftTimeline,
    #[sea_orm(
        belongs_to = "super::project_timeline::Entity",
        from = "Column::DeployedTimeline",
        to = "super::project_timeline::Column::Id"
    )]
    DeployedTimeline,
}

impl Related<super::project::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Project.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

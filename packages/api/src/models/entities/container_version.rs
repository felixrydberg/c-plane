use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "project_container_version")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub container_id: Uuid,
    pub organization_id: Uuid,
    pub version: i32,
    pub image: String,
    pub public: bool,
    pub replica_count: i32,
    pub port: Option<i32>,
    pub env: Option<serde_json::Value>,
    pub resources: Option<serde_json::Value>,
    pub pull_secret_id: Option<Uuid>,
    pub health_check: Option<serde_json::Value>,
    pub created_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::container::Entity",
        from = "Column::ContainerId",
        to = "super::container::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Container,
}

impl Related<super::container::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Container.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

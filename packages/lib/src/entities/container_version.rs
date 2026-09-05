use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "container_version")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub container_id: Uuid,
    pub organization_id: Uuid,
    pub version: i32,
    pub image: String,
    pub resolved_image: String,
    pub public: bool,
    pub replica_count: i32,
    pub port: Option<i32>,
    pub env: Option<serde_json::Value>,
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub external_registry_id: Option<Uuid>,
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
    #[sea_orm(
        belongs_to = "super::external_registry::Entity",
        from = "Column::ExternalRegistryId",
        to = "super::external_registry::Column::Id",
        on_update = "Cascade",
        on_delete = "Restrict"
    )]
    ExternalRegistry,
}

impl Related<super::container::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Container.def()
    }
}

impl Related<super::external_registry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ExternalRegistry.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

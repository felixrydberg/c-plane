use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "container")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub project_id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub region_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::container_version::Entity")]
    Versions,
}

impl Related<super::container_version::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Versions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

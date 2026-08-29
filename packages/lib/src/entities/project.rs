use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "project")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub organization_id: Uuid,
    pub default_environment_id: Option<Uuid>,
    pub name: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organisation::Entity",
        from = "Column::OrganizationId",
        to = "super::organisation::Column::Id"
    )]
    Organization,
    #[sea_orm(
        belongs_to = "super::project_environment::Entity",
        from = "Column::DefaultEnvironmentId",
        to = "super::project_environment::Column::Id"
    )]
    DefaultEnvironment,
    #[sea_orm(has_many = "super::project_environment::Entity")]
    Environments,
    #[sea_orm(has_many = "super::project_timeline::Entity")]
    Timelines,
}

impl Related<super::organisation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organization.def()
    }
}

impl Related<super::project_environment::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::DefaultEnvironment.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

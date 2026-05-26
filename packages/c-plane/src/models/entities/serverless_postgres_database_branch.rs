use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "serverless_postgres_database_branch")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub database_id: Uuid,
    pub branch_id: Uuid,
    pub organization_id: Uuid,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::serverless_postgres_database::Entity",
        from = "Column::DatabaseId",
        to = "super::serverless_postgres_database::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Database,
    #[sea_orm(
        belongs_to = "super::project_branch::Entity",
        from = "Column::BranchId",
        to = "super::project_branch::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Branch,
}

impl Related<super::serverless_postgres_database::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Database.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

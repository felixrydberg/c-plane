use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "managed_registry_gc_runs")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    pub organization_id: Uuid,
    pub started_at: DateTimeWithTimeZone,
    pub finished_at: DateTimeWithTimeZone,
    pub bytes_before: Option<i64>,
    pub bytes_after: Option<i64>,
    pub result: String,
    pub error: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::managed_registry::Entity",
        from = "Column::OrganizationId",
        to = "super::managed_registry::Column::OrganizationId",
        on_delete = "Cascade"
    )]
    ManagedRegistry,
}

impl Related<super::managed_registry::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ManagedRegistry.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

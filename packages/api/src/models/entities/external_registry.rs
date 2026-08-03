use sea_orm::entity::prelude::*;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq)]
#[sea_orm(table_name = "external_registry")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub host: String,
    pub username: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::container_version::Entity")]
    ContainerVersions,
}

impl Related<super::container_version::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ContainerVersions.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

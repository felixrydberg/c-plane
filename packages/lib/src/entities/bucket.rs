use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "bucket")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub region_id: Uuid,
    pub sse_secret_id: Uuid,
    pub status: BucketStatus,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "foundation_bucket_status")]
pub enum BucketStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "deleting")]
    Deleting,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::region::Entity",
        from = "Column::RegionId",
        to = "super::region::Column::Id",
        on_delete = "Restrict"
    )]
    Region,
    #[sea_orm(
        belongs_to = "super::secret::Entity",
        from = "Column::SseSecretId",
        to = "super::secret::Column::Id",
        on_delete = "Restrict"
    )]
    SseSecret,
}

impl Related<super::region::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Region.def()
    }
}

impl Related<super::secret::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SseSecret.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

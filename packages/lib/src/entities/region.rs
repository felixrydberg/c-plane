use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "regions")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub slug: String,
    pub display_name: String,
    pub s3_provider_id: Option<Uuid>,
    pub status: RegionStatus,
    pub routing_mode: RegionRoutingMode,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "region_status")]
pub enum RegionStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "inactive")]
    Inactive,
    #[sea_orm(string_value = "maintenance")]
    Maintenance,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "Enum",
    enum_name = "region_routing_mode"
)]
pub enum RegionRoutingMode {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "draining")]
    Draining,
    #[sea_orm(string_value = "disabled")]
    Disabled,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::s3_provider::Entity",
        from = "Column::S3ProviderId",
        to = "super::s3_provider::Column::Id",
        on_delete = "Restrict"
    )]
    S3Provider,
}

impl Related<super::s3_provider::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::S3Provider.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

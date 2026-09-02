use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "bucket")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub s3_provider_id: Uuid,
    pub sse_secret_id: Uuid,
    pub status: BucketStatus,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(
    rs_type = "String",
    db_type = "Enum",
    enum_name = "foundation_bucket_status"
)]
pub enum BucketStatus {
    #[sea_orm(string_value = "active")]
    Active,
    #[sea_orm(string_value = "deleting")]
    Deleting,
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
    #[sea_orm(
        belongs_to = "super::secret::Entity",
        from = "Column::SseSecretId",
        to = "super::secret::Column::Id",
        on_delete = "Restrict"
    )]
    SseSecret,
    #[sea_orm(has_many = "super::bucket_grant::Entity")]
    BucketGrant,
}

impl Related<super::s3_provider::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::S3Provider.def()
    }
}

impl Related<super::secret::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::SseSecret.def()
    }
}

impl Related<super::bucket_grant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::BucketGrant.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

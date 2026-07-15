use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "storage_access_token_bucket")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub access_token_id: Uuid,
    #[sea_orm(primary_key, auto_increment = false)]
    pub bucket_id: Uuid,
    pub organization_id: Uuid,
    pub can_read: bool,
    pub can_write: bool,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::storage_access_token::Entity",
        from = "Column::AccessTokenId",
        to = "super::storage_access_token::Column::Id",
        on_delete = "Cascade"
    )]
    AccessToken,
    #[sea_orm(
        belongs_to = "super::bucket::Entity",
        from = "Column::BucketId",
        to = "super::bucket::Column::Id",
        on_delete = "Cascade"
    )]
    Bucket,
}

impl ActiveModelBehavior for ActiveModel {}

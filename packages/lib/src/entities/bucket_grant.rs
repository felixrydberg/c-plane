use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "bucket_grant")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub credential_id: Uuid,
    pub bucket_id: Uuid,
    pub organization_id: Option<Uuid>,
    pub prefix: String,
    pub can_read: bool,
    pub can_write: bool,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::credential::Entity",
        from = "Column::CredentialId",
        to = "super::credential::Column::Id",
        on_delete = "Cascade"
    )]
    Credential,
    #[sea_orm(
        belongs_to = "super::storage::Entity",
        from = "Column::BucketId",
        to = "super::storage::Column::BucketId",
        on_delete = "Restrict"
    )]
    Storage,
}

impl Related<super::credential::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Credential.def()
    }
}

impl Related<super::storage::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Storage.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

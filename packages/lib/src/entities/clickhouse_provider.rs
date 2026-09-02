use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "clickhouse_providers")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub name: String,
    pub endpoint_url: String,
    pub cluster_name: String,
    pub credential_secret_id: Uuid,
    pub bucket_id: Uuid,
    pub storage_credential_id: Uuid,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::secret::Entity",
        from = "Column::CredentialSecretId",
        to = "super::secret::Column::Id",
        on_delete = "Restrict"
    )]
    Secret,
    #[sea_orm(
        belongs_to = "super::bucket::Entity",
        from = "Column::BucketId",
        to = "super::bucket::Column::Id",
        on_delete = "Restrict"
    )]
    Bucket,
    #[sea_orm(
        belongs_to = "super::credential::Entity",
        from = "Column::StorageCredentialId",
        to = "super::credential::Column::Id",
        on_delete = "Restrict"
    )]
    StorageCredential,
}

impl ActiveModelBehavior for ActiveModel {}

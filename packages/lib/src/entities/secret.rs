use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "secret")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub scope: SecretScope,
    pub organization_id: Option<Uuid>,
    pub ciphertext: String,
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "secret_scope")]
pub enum SecretScope {
    #[sea_orm(string_value = "platform")]
    Platform,
    #[sea_orm(string_value = "tenant")]
    Tenant,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "organisation_members")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: Uuid,
    pub organisation_id: Uuid,
    pub identity_id: Uuid, // References Ory Kratos identity ID directly
    pub role: OrganisationRole,
    pub is_active: bool,
    pub joined_at: DateTime,
    pub invited_by: Uuid, // References Ory Kratos identity ID of inviter
    pub invited_at: DateTime,
    pub invitation_accepted_at: DateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, EnumIter, DeriveActiveEnum, Serialize, Deserialize)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "organisation_role")]
pub enum OrganisationRole {
    #[sea_orm(string_value = "owner")]
    Owner,
    #[sea_orm(string_value = "admin")]
    Admin,
    #[sea_orm(string_value = "member")]
    Member,
    #[sea_orm(string_value = "viewer")]
    Viewer,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::organisation::Entity",
        from = "Column::OrganisationId",
        to = "super::organisation::Column::Id",
        on_update = "Cascade",
        on_delete = "Cascade"
    )]
    Organisation,
}

impl Related<super::organisation::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Organisation.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

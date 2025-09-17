use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, EntityTrait, Set, TransactionTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::{AppError, DatabaseError, OrganisationError};
use crate::models::entities::{Organisation, OrganisationActiveModel, OrganisationModel};
use crate::models::entities::{
    OrganisationMember, OrganisationMemberActiveModel, OrganisationMemberModel, OrganisationRole,
};

#[derive(Serialize, Deserialize)]
pub struct CreateOrganisationData {
    pub name: String,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub identity_id: Uuid,
}

pub async fn create_organisation(
    db: DatabaseConnection,
    data: CreateOrganisationData,
) -> Result<(OrganisationModel, OrganisationMemberModel), AppError> {
    let now = chrono::Utc::now().naive_utc();
    let created_by = data.identity_id;
    let (organisation, organisation_member) = db
        .transaction::<_, (OrganisationModel, OrganisationMemberModel), DbErr>(|transaction| {
            Box::pin(async move {
                let uuid = Uuid::new_v4();
                let organisation = OrganisationActiveModel {
                    id: Set(uuid),
                    name: Set(data.name.clone()),
                    description: Set(data.description.clone()),
                    created_at: Set(now),
                    updated_at: Set(now),
                    created_by: Set(created_by),
                    avatar_url: Set(data.avatar_url.clone()),
                    is_active: Set(true),
                };
                let organisation: OrganisationModel = organisation.insert(transaction).await?;

                let organisation_member = OrganisationMemberActiveModel {
                    id: Set(Uuid::new_v4()),
                    organisation_id: Set(uuid),
                    identity_id: Set(created_by),
                    role: Set(OrganisationRole::Owner),
                    is_active: Set(true),
                    joined_at: Set(now),
                    invited_by: Set(created_by),
                    invited_at: Set(now),
                    invitation_accepted_at: Set(now),
                };
                let organisation_member: OrganisationMemberModel =
                    organisation_member.insert(transaction).await?;

                Ok((organisation, organisation_member))
            })
        })
        .await
        .map_err(|e| AppError::Database(DatabaseError::TransactionFailed(e.to_string())))?;

    Ok((organisation, organisation_member))
}

pub async fn get_organisation(
    db: DatabaseConnection,
    organisation_id: Uuid,
) -> Result<OrganisationModel, AppError> {
    let organisation = Organisation::find_by_id(organisation_id)
        .one(&db)
        .await?
        .ok_or_else(|| {
            AppError::Organisation(OrganisationError::OrganisationNotFound(organisation_id))
        })?;

    Ok(organisation)
}

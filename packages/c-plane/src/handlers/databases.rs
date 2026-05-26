use sea_orm::{EntityTrait, QueryFilter, ColumnTrait};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use utoipa::ToSchema;

use crate::errors::AppError;
use crate::models::entities::project;
use crate::state::TenantDatabase;

#[derive(Deserialize, ToSchema)]
pub struct CreateDatabaseRequest {
    pub name: String,
    pub project_id: Uuid,
    pub cpu: Option<String>,
    pub ram: Option<String>,
    #[serde(default)]
    pub high_availability: bool,
    pub read_replicas: Option<i32>,
    #[serde(default)]
    pub autoscaling_enabled: bool,
    pub autoscaling_min_cpu: Option<String>,
    pub autoscaling_max_cpu: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct ListDatabasesQuery {
    pub project_id: Uuid,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateDatabaseRequest {
    pub name: Option<String>,
    pub cpu: Option<String>,
    pub ram: Option<String>,
    pub high_availability: Option<bool>,
    pub read_replicas: Option<i32>,
    pub autoscaling_enabled: Option<bool>,
    pub autoscaling_min_cpu: Option<String>,
    pub autoscaling_max_cpu: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateDatabaseBranchRequest {
    pub branch_id: Uuid,
}

#[derive(Serialize, ToSchema)]
pub struct DatabaseBranchResponse {
    pub id: Uuid,
    pub database_id: Uuid,
    pub branch_id: Uuid,
    pub organization_id: Uuid,
}

#[derive(Serialize, ToSchema)]
pub struct DatabaseResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub cpu: Option<String>,
    pub ram: Option<String>,
    pub high_availability: bool,
    pub read_replicas: Option<i32>,
    pub autoscaling_enabled: bool,
    pub autoscaling_min_cpu: Option<String>,
    pub autoscaling_max_cpu: Option<String>,
    pub default_branch_id: Option<Uuid>,
}

pub fn verify_org_access(tenant_db: &TenantDatabase, org_id: Uuid) -> Result<(), AppError> {
    if !tenant_db.context.allowed_organizations.contains(&org_id) {
        return Err(AppError::Forbidden("You do not have access to this organization".into()));
    }
    Ok(())
}

pub async fn verify_project_in_org(
    tx: &impl sea_orm::ConnectionTrait,
    project_id: Uuid,
    organization_id: Uuid,
) -> Result<(), AppError> {
    let exists = project::Entity::find()
        .filter(project::Column::Id.eq(project_id))
        .filter(project::Column::OrganizationId.eq(organization_id))
        .one(tx)
        .await?
        .is_some();

    if !exists {
        return Err(AppError::NotFound("Project not found in this organization".into()));
    }
    Ok(())
}

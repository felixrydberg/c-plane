use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::entities::project;
use crate::state::TenantDatabase;

#[derive(Deserialize, ToSchema)]
pub struct CreateDatabaseRequest {
    pub name: String,
    pub project_id: Uuid,
    pub region_id: Uuid,
    pub backup_retention_days: Option<i32>,
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
}

#[derive(Deserialize, ToSchema)]
pub struct CreateDatabaseBranchRequest {
    pub branch_id: Uuid,
    pub backup_retention_days: Option<i32>,
    pub cpu: Option<String>,
    pub ram: Option<String>,
    #[serde(default)]
    pub high_availability: Option<bool>,
    pub read_replicas: Option<i32>,
    #[serde(default)]
    pub autoscaling_enabled: Option<bool>,
    pub autoscaling_min_cpu: Option<String>,
    pub autoscaling_max_cpu: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateDatabaseBranchRequest {
    pub backup_retention_days: Option<Option<i32>>,
    pub cpu: Option<String>,
    pub ram: Option<String>,
    pub high_availability: Option<bool>,
    pub read_replicas: Option<i32>,
    pub autoscaling_enabled: Option<bool>,
    pub autoscaling_min_cpu: Option<String>,
    pub autoscaling_max_cpu: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct DatabaseBranchResponse {
    pub id: Uuid,
    pub database_id: Uuid,
    pub branch_id: Uuid,
    pub organization_id: Uuid,
    pub backup_retention_days: Option<i32>,
    pub cpu: Option<String>,
    pub ram: Option<String>,
    pub high_availability: bool,
    pub read_replicas: Option<i32>,
    pub autoscaling_enabled: bool,
    pub autoscaling_min_cpu: Option<String>,
    pub autoscaling_max_cpu: Option<String>,
    pub organization_region_backup_bucket_id: Uuid,
    pub backup_credential_id: Uuid,
}

#[derive(Serialize, ToSchema)]
pub struct DatabaseResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub default_branch_id: Option<Uuid>,
    pub region_id: Uuid,
}

#[derive(Serialize, ToSchema)]
pub struct DatabaseWithBranchesResponse {
    #[serde(flatten)]
    pub database: DatabaseResponse,
    pub branches: Vec<DatabaseBranchResponse>,
}

#[allow(unused_imports)]
pub use lib::services::postgres_databases::{
    validate_autoscaling, validate_backup_retention_days, validate_cpu, validate_ram,
    validate_read_replicas,
};

pub fn verify_org_access(tenant_db: &TenantDatabase, org_id: Uuid) -> Result<(), AppError> {
    if !tenant_db.context.allowed_organizations.contains(&org_id) {
        return Err(AppError::Forbidden(
            "You do not have access to this organization".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_backup_retention_days;

    #[test]
    fn backup_retention_must_be_positive_when_enabled() {
        assert!(validate_backup_retention_days(None).is_ok());
        assert!(validate_backup_retention_days(Some(1)).is_ok());
        assert!(validate_backup_retention_days(Some(0)).is_err());
    }

    #[test]
    fn sizing_validators_reject_garbage() {
        use super::{validate_autoscaling, validate_cpu, validate_ram, validate_read_replicas};

        assert!(validate_cpu("0.5").is_ok());
        assert!(validate_cpu(" 2 ").is_ok());
        assert!(validate_cpu("").is_err());
        assert!(validate_cpu("big").is_err());
        assert!(validate_cpu("-1").is_err());
        assert!(validate_cpu("65").is_err());

        assert!(validate_ram("1024Mi").is_ok());
        assert!(validate_ram("1024").is_ok());
        assert!(validate_ram("0").is_err());
        assert!(validate_ram("1GiB").is_err());
        assert!(validate_ram("65537").is_err());

        assert!(validate_read_replicas(0).is_ok());
        assert!(validate_read_replicas(-1).is_err());

        assert!(validate_autoscaling(Some("0.25"), Some("2")).is_ok());
        assert!(validate_autoscaling(Some("2"), Some("0.25")).is_err());
        assert!(validate_autoscaling(Some("2"), None).is_ok());
        assert!(validate_autoscaling(Some("not-a-cpu"), None).is_err());
        assert!(validate_autoscaling(None, Some("NaN")).is_err());
        assert!(validate_autoscaling(Some("65"), None).is_err());
    }
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
        return Err(AppError::NotFound(
            "Project not found in this organization".into(),
        ));
    }
    Ok(())
}

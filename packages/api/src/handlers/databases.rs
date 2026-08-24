use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::entities::project;
use crate::state::{OrganizationContext, TenantDatabase};

#[derive(Deserialize, ToSchema)]
pub struct CreateDatabaseRequest {
    pub name: String,
    pub project_id: Uuid,
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
}

#[derive(Serialize, ToSchema)]
pub struct DatabaseResponse {
    pub id: Uuid,
    pub project_id: Uuid,
    pub name: String,
    pub default_branch_id: Option<Uuid>,
}

pub fn validate_backup_retention_days(retention_days: Option<i32>) -> Result<(), AppError> {
    if retention_days.is_some_and(|days| days <= 0) {
        return Err(AppError::BadRequest(
            "Backup retention must be a positive number of days or disabled".into(),
        ));
    }
    Ok(())
}

pub fn verify_org_access(tenant_db: &TenantDatabase, org_id: Uuid) -> Result<(), AppError> {
    if !tenant_db.context.allowed_organizations.contains(&org_id) {
        return Err(AppError::Forbidden(
            "You do not have access to this organization".into(),
        ));
    }
    Ok(())
}

pub fn verify_org_owner(tenant_db: &TenantDatabase, org_id: Uuid) -> Result<(), AppError> {
    verify_org_access(tenant_db, org_id)?;
    require_role(&tenant_db.context, org_id)
}

fn require_role(context: &OrganizationContext, org_id: Uuid) -> Result<(), AppError> {
    if let Some(api_key_organization_id) = context.api_key_organization_id {
        return if api_key_organization_id == org_id {
            Ok(())
        } else {
            Err(AppError::Forbidden(
                "API key is not owned by this organization".into(),
            ))
        };
    }

    match context.organization_roles.get(&org_id).map(String::as_str) {
        Some("owner") => Ok(()),
        _ => Err(AppError::Forbidden(
            "Organization owner role required".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{require_role, validate_backup_retention_days};
    use crate::state::OrganizationContext;
    use std::collections::HashMap;
    use uuid::Uuid;

    #[test]
    fn backup_retention_must_be_positive_when_enabled() {
        assert!(validate_backup_retention_days(None).is_ok());
        assert!(validate_backup_retention_days(Some(1)).is_ok());
        assert!(validate_backup_retention_days(Some(0)).is_err());
    }

    fn context_with(org_id: Uuid, role: Option<&str>) -> OrganizationContext {
        OrganizationContext {
            allowed_organizations: vec![org_id],
            organization_roles: role
                .map(|r| HashMap::from([(org_id, r.to_string())]))
                .unwrap_or_default(),
            api_key_organization_id: None,
        }
    }

    #[test]
    fn api_key_organization_ownership_is_checked_separately_from_member_roles() {
        let org = Uuid::new_v4();
        let context = OrganizationContext {
            allowed_organizations: vec![org],
            organization_roles: HashMap::from([(Uuid::new_v4(), "owner".to_string())]),
            api_key_organization_id: Some(org),
        };

        assert!(require_role(&context, org).is_ok());
        assert!(require_role(&context, Uuid::new_v4()).is_err());
    }

    #[test]
    fn owner_role_passes_and_member_or_absent_role_fails() {
        let org = Uuid::new_v4();
        assert!(require_role(&context_with(org, Some("owner")), org).is_ok());
        assert!(require_role(&context_with(org, Some("member")), org).is_err());
        assert!(require_role(&context_with(org, None), org).is_err());
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

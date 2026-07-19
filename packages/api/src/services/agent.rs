use uuid::Uuid;

use crate::errors::AppError;

pub async fn emit_compute(
    project_id: Uuid,
    organization_id: Uuid,
    environment_id: Uuid,
    revision_id: Uuid,
) -> Result<(), AppError> {
    tracing::info!(
        project_id = %project_id,
        organization_id = %organization_id,
        environment_id = %environment_id,
        revision_id = %revision_id,
        "emit_compute called"
    );
    Ok(())
}

pub async fn emit_postgres_branch(
    database_id: Uuid,
    organization_id: Uuid,
    branch_id: Uuid,
    database_branch_id: Uuid,
) -> Result<(), AppError> {
    tracing::info!(
        database_id = %database_id,
        organization_id = %organization_id,
        branch_id = %branch_id,
        database_branch_id = %database_branch_id,
        "emit_postgres_branch called"
    );
    Ok(())
}

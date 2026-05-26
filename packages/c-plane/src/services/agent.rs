use uuid::Uuid;

use crate::errors::AppError;

pub async fn emit_project(
    project_id: Uuid,
    organization_id: Uuid,
    branch_id: Uuid,
    timeline_id: Uuid,
) -> Result<(), AppError> {
    tracing::info!(
        project_id = %project_id,
        organization_id = %organization_id,
        branch_id = %branch_id,
        timeline_id = %timeline_id,
        "emit_project called"
    );
    Ok(())
}

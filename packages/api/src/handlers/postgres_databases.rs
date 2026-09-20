use axum::{Json, extract::Path, http::StatusCode};
use uuid::Uuid;

use super::databases::{
    CreateDatabaseBranchRequest, CreateDatabaseRequest, DatabaseBranchResponse, DatabaseResponse,
    DatabaseWithBranchesResponse, ListDatabasesQuery, UpdateDatabaseBranchRequest,
    UpdateDatabaseRequest,
};
use crate::errors::AppError;
use crate::middleware::auth::AuthContext;
use crate::models::entities::{postgres_database, postgres_database_branch};
use crate::services::postgres_databases as database_service;

fn db_to_response(db: &postgres_database::Model) -> DatabaseResponse {
    DatabaseResponse {
        id: db.id,
        project_id: db.project_id,
        name: db.name.clone(),
        default_branch_id: db.default_branch_id,
        region_id: db.region_id,
    }
}

fn branch_to_response(b: &postgres_database_branch::Model) -> DatabaseBranchResponse {
    DatabaseBranchResponse {
        id: b.id,
        database_id: b.database_id,
        branch_id: b.branch_id,
        organization_id: b.organization_id,
        backup_retention_days: b.backup_retention_days,
        cpu: b.cpu.clone(),
        ram: b.ram.clone(),
        high_availability: b.high_availability,
        read_replicas: b.read_replicas,
        autoscaling_enabled: b.autoscaling_enabled,
        autoscaling_min_cpu: b.autoscaling_min_cpu.clone(),
        autoscaling_max_cpu: b.autoscaling_max_cpu.clone(),
        organization_region_backup_bucket_id: b.organization_region_backup_bucket_id,
        backup_credential_id: b.backup_credential_id,
    }
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/databases/postgres",
    request_body = CreateDatabaseRequest,
    params(("organization_id" = Uuid, Path, description = "Organization ID")),
    responses(
        (status = 201, description = "Database created", body = DatabaseResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "databases/postgres",
)]
pub async fn create_database(
    AuthContext { tenant_db, auth }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Json(body): Json<CreateDatabaseRequest>,
) -> Result<(StatusCode, Json<DatabaseResponse>), AppError> {
    let database = database_service::create_database(
        &tenant_db,
        organization_id,
        auth.actor_id,
        database_service::CreateDatabaseInput {
            name: body.name,
            project_id: body.project_id,
            region_id: body.region_id,
            backup_retention_days: body.backup_retention_days,
            cpu: body.cpu,
            ram: body.ram,
            high_availability: body.high_availability,
            read_replicas: body.read_replicas,
            autoscaling_enabled: body.autoscaling_enabled,
            autoscaling_min_cpu: body.autoscaling_min_cpu,
            autoscaling_max_cpu: body.autoscaling_max_cpu,
        },
    )
    .await?;
    Ok((StatusCode::CREATED, Json(db_to_response(&database))))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/databases/postgres",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("project_id" = Uuid, Query, description = "Project ID"),
    ),
    responses((status = 200, description = "Project databases with branches", body = Vec<DatabaseWithBranchesResponse>)),
    tag = "databases/postgres",
)]
pub async fn list_databases(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
    axum::extract::Query(query): axum::extract::Query<ListDatabasesQuery>,
) -> Result<Json<Vec<DatabaseWithBranchesResponse>>, AppError> {
    let databases =
        database_service::list_databases(&tenant_db, organization_id, query.project_id).await?;
    Ok(Json(
        databases
            .into_iter()
            .map(|(database, branches)| DatabaseWithBranchesResponse {
                database: db_to_response(&database),
                branches: branches.iter().map(branch_to_response).collect(),
            })
            .collect(),
    ))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/databases/postgres/{database_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("database_id" = Uuid, Path, description = "Database ID"),
    ),
    responses(
        (status = 200, description = "Database details", body = DatabaseResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "databases/postgres",
)]
pub async fn get_database(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, database_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DatabaseResponse>, AppError> {
    let database = database_service::get_database(&tenant_db, organization_id, database_id).await?;
    Ok(Json(db_to_response(&database)))
}

#[utoipa::path(
    patch,
    path = "/api/organization/{organization_id}/databases/postgres/{database_id}",
    request_body = UpdateDatabaseRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("database_id" = Uuid, Path, description = "Database ID"),
    ),
    responses(
        (status = 200, description = "Database updated", body = DatabaseResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "databases/postgres",
)]
pub async fn update_database(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, database_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateDatabaseRequest>,
) -> Result<Json<DatabaseResponse>, AppError> {
    let database = database_service::update_database(
        &tenant_db,
        organization_id,
        auth.actor_id,
        database_id,
        database_service::UpdateDatabaseInput { name: body.name },
    )
    .await?;
    Ok(Json(db_to_response(&database)))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/databases/postgres/{database_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("database_id" = Uuid, Path, description = "Database ID"),
    ),
    responses(
        (status = 200, description = "Database deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "databases/postgres",
)]
pub async fn delete_database(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, database_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    database_service::delete_database(&tenant_db, organization_id, auth.actor_id, database_id)
        .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

#[utoipa::path(
    get,
    path = "/api/organization/{organization_id}/databases/postgres/{database_id}/branches",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("database_id" = Uuid, Path, description = "Database ID"),
    ),
    responses(
        (status = 200, description = "Database branch links", body = Vec<DatabaseBranchResponse>),
        (status = 404, description = "Not found"),
    ),
    tag = "databases/postgres",
)]
pub async fn list_database_branches(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, database_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<DatabaseBranchResponse>>, AppError> {
    let branches =
        database_service::list_database_branches(&tenant_db, organization_id, database_id).await?;
    Ok(Json(branches.iter().map(branch_to_response).collect()))
}

#[utoipa::path(
    patch,
    path = "/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}",
    request_body = UpdateDatabaseBranchRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("database_id" = Uuid, Path, description = "Database ID"),
        ("branch_id" = Uuid, Path, description = "Branch ID"),
    ),
    responses(
        (status = 200, description = "Database branch updated", body = DatabaseBranchResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "databases/postgres",
)]
pub async fn update_database_branch(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, database_id, branch_id)): Path<(Uuid, Uuid, Uuid)>,
    Json(body): Json<UpdateDatabaseBranchRequest>,
) -> Result<Json<DatabaseBranchResponse>, AppError> {
    let branch = database_service::update_database_branch(
        &tenant_db,
        organization_id,
        auth.actor_id,
        database_id,
        branch_id,
        database_service::UpdateDatabaseBranchInput {
            backup_retention_days: body.backup_retention_days,
            cpu: body.cpu,
            ram: body.ram,
            high_availability: body.high_availability,
            read_replicas: body.read_replicas,
            autoscaling_enabled: body.autoscaling_enabled,
            autoscaling_min_cpu: body.autoscaling_min_cpu,
            autoscaling_max_cpu: body.autoscaling_max_cpu,
        },
    )
    .await?;
    Ok(Json(branch_to_response(&branch)))
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/databases/postgres/{database_id}/branches",
    request_body = CreateDatabaseBranchRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("database_id" = Uuid, Path, description = "Database ID"),
    ),
    responses(
        (status = 201, description = "Database branch link created", body = DatabaseBranchResponse),
        (status = 200, description = "Link already exists", body = DatabaseBranchResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "databases/postgres",
)]
pub async fn create_database_branch(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, database_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateDatabaseBranchRequest>,
) -> Result<(StatusCode, Json<DatabaseBranchResponse>), AppError> {
    let result = database_service::create_database_branch(
        &tenant_db,
        organization_id,
        auth.actor_id,
        database_id,
        database_service::CreateDatabaseBranchInput {
            branch_id: body.branch_id,
            backup_retention_days: body.backup_retention_days,
            cpu: body.cpu,
            ram: body.ram,
            high_availability: body.high_availability,
            read_replicas: body.read_replicas,
            autoscaling_enabled: body.autoscaling_enabled,
            autoscaling_min_cpu: body.autoscaling_min_cpu,
            autoscaling_max_cpu: body.autoscaling_max_cpu,
        },
    )
    .await?;
    let status = if result.created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    Ok((status, Json(branch_to_response(&result.row))))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/databases/postgres/{database_id}/branches/{branch_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("database_id" = Uuid, Path, description = "Database ID"),
        ("branch_id" = Uuid, Path, description = "Branch ID"),
    ),
    responses(
        (status = 200, description = "Database branch link deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "databases/postgres",
)]
pub async fn delete_database_branch(
    AuthContext { tenant_db, auth }: AuthContext,
    Path((organization_id, database_id, branch_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    database_service::delete_branch(
        &tenant_db,
        organization_id,
        auth.actor_id,
        database_id,
        branch_id,
    )
    .await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

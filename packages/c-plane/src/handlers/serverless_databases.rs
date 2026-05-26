use axum::{Json, extract::Path};
use sea_orm::{Set, EntityTrait, ActiveModelTrait, QueryFilter, ColumnTrait, QueryOrder};
use uuid::Uuid;

use super::databases::{
    CreateDatabaseRequest, DatabaseResponse, ListDatabasesQuery, UpdateDatabaseRequest,
    CreateDatabaseBranchRequest, DatabaseBranchResponse,
    verify_org_access, verify_project_in_org,
};
use crate::errors::AppError;
use crate::models::entities::{
    serverless_postgres_database, serverless_postgres_database_branch, project_branch,
};
use crate::middleware::auth::AuthContext;

fn to_response(db: &serverless_postgres_database::Model) -> DatabaseResponse {
    DatabaseResponse {
        id: db.id,
        project_id: db.project_id,
        name: db.name.clone(),
        cpu: db.cpu.clone(),
        ram: db.ram.clone(),
        high_availability: db.high_availability,
        read_replicas: db.read_replicas,
        autoscaling_enabled: db.autoscaling_enabled,
        autoscaling_min_cpu: db.autoscaling_min_cpu.clone(),
        autoscaling_max_cpu: db.autoscaling_max_cpu.clone(),
        default_branch_id: db.default_branch_id,
    }
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/databases/serverless",
    request_body = CreateDatabaseRequest,
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
    ),
    responses(
        (status = 201, description = "Database created", body = DatabaseResponse),
        (status = 404, description = "Not found"),
    ),
    tag = "databases/serverless",
)]
pub async fn create_database(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
    Json(body): Json<CreateDatabaseRequest>,
) -> Result<(axum::http::StatusCode, Json<DatabaseResponse>), AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let name = body.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Project(crate::errors::project::ProjectError::InvalidSlug("Name is required".into())));
    }

    let db_id = Uuid::new_v4();
    let db_branch_id = Uuid::new_v4();

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    verify_project_in_org(tx, body.project_id, organization_id).await?;

    let main_branch = project_branch::Entity::find()
        .filter(project_branch::Column::ProjectId.eq(body.project_id))
        .filter(project_branch::Column::Name.eq("main"))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Main branch not found".into()))?;

    let created: serverless_postgres_database::Model = serverless_postgres_database::ActiveModel {
        id: Set(db_id),
        project_id: Set(body.project_id),
        organization_id: Set(organization_id),
        default_branch_id: Set(None),
        name: Set(name),
        cpu: Set(body.cpu),
        ram: Set(body.ram),
        high_availability: Set(body.high_availability),
        read_replicas: Set(body.read_replicas),
        autoscaling_enabled: Set(body.autoscaling_enabled),
        autoscaling_min_cpu: Set(body.autoscaling_min_cpu),
        autoscaling_max_cpu: Set(body.autoscaling_max_cpu),
    }.insert(tx).await?;

    let _db_branch: serverless_postgres_database_branch::Model = serverless_postgres_database_branch::ActiveModel {
        id: Set(db_branch_id),
        database_id: Set(db_id),
        branch_id: Set(main_branch.id),
        organization_id: Set(organization_id),
    }.insert(tx).await?;

    let mut db_active: serverless_postgres_database::ActiveModel = created.clone().into();
    db_active.default_branch_id = Set(Some(db_branch_id));
    let updated = db_active.update(tx).await?;

    scoped.commit().await?;

    Ok((axum::http::StatusCode::CREATED, Json(to_response(&updated))))
}

pub async fn list_databases(
    AuthContext { tenant_db, .. }: AuthContext,
    Path(organization_id): Path<Uuid>,
    axum::extract::Query(query): axum::extract::Query<ListDatabasesQuery>,
) -> Result<Json<Vec<DatabaseResponse>>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    verify_project_in_org(tx, query.project_id, organization_id).await?;

    use serverless_postgres_database::{Entity, Column};
    let dbs = Entity::find()
        .filter(Column::ProjectId.eq(query.project_id))
        .order_by_asc(Column::Name)
        .all(tx)
        .await?;

    scoped.commit().await?;

    Ok(Json(dbs.iter().map(to_response).collect()))
}

pub async fn get_database(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, database_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<DatabaseResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    use serverless_postgres_database::Entity;
    let db = Entity::find_by_id(database_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Serverless database not found".into()))?;

    verify_project_in_org(tx, db.project_id, organization_id).await?;

    scoped.commit().await?;

    Ok(Json(to_response(&db)))
}

pub async fn update_database(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, database_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateDatabaseRequest>,
) -> Result<Json<DatabaseResponse>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    use serverless_postgres_database::Entity;
    let db = Entity::find_by_id(database_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Serverless database not found".into()))?;

    verify_project_in_org(tx, db.project_id, organization_id).await?;

    let mut active: serverless_postgres_database::ActiveModel = db.into();
    if let Some(ref name) = body.name {
        active.name = Set(name.trim().to_string());
    }
    if body.cpu.is_some() {
        active.cpu = Set(body.cpu);
    }
    if body.ram.is_some() {
        active.ram = Set(body.ram);
    }
    if let Some(v) = body.high_availability {
        active.high_availability = Set(v);
    }
    if body.read_replicas.is_some() {
        active.read_replicas = Set(body.read_replicas);
    }
    if let Some(v) = body.autoscaling_enabled {
        active.autoscaling_enabled = Set(v);
    }
    if body.autoscaling_min_cpu.is_some() {
        active.autoscaling_min_cpu = Set(body.autoscaling_min_cpu);
    }
    if body.autoscaling_max_cpu.is_some() {
        active.autoscaling_max_cpu = Set(body.autoscaling_max_cpu);
    }

    let updated = active.update(tx).await?;
    scoped.commit().await?;

    Ok(Json(to_response(&updated)))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/databases/serverless/{database_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("database_id" = Uuid, Path, description = "Database ID"),
    ),
    responses(
        (status = 200, description = "Database deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "databases/serverless",
)]
pub async fn delete_database(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, database_id)): Path<(Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    use serverless_postgres_database::Entity;
    let db = Entity::find_by_id(database_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Serverless database not found".into()))?;

    verify_project_in_org(tx, db.project_id, organization_id).await?;

    Entity::delete_by_id(database_id).exec(tx).await?;
    scoped.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

#[utoipa::path(
    post,
    path = "/api/organization/{organization_id}/databases/serverless/{database_id}/branches",
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
    tag = "databases/serverless",
)]
pub async fn create_database_branch(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, database_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateDatabaseBranchRequest>,
) -> Result<(axum::http::StatusCode, Json<DatabaseBranchResponse>), AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let db = serverless_postgres_database::Entity::find_by_id(database_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Serverless database not found".into()))?;

    verify_project_in_org(tx, db.project_id, organization_id).await?;

    let branch = project_branch::Entity::find_by_id(body.branch_id)
        .filter(project_branch::Column::ProjectId.eq(db.project_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Branch not found in database's project".into()))?;

    let existing = serverless_postgres_database_branch::Entity::find()
        .filter(serverless_postgres_database_branch::Column::DatabaseId.eq(database_id))
        .filter(serverless_postgres_database_branch::Column::BranchId.eq(body.branch_id))
        .one(tx)
        .await?;

    if let Some(row) = existing {
        return Ok((axum::http::StatusCode::OK, Json(DatabaseBranchResponse {
            id: row.id,
            database_id: row.database_id,
            branch_id: row.branch_id,
            organization_id: row.organization_id,
        })));
    }

    let id = Uuid::new_v4();
    let row: serverless_postgres_database_branch::Model = serverless_postgres_database_branch::ActiveModel {
        id: Set(id),
        database_id: Set(database_id),
        branch_id: Set(branch.id),
        organization_id: Set(organization_id),
    }.insert(tx).await?;

    scoped.commit().await?;

    Ok((axum::http::StatusCode::CREATED, Json(DatabaseBranchResponse {
        id: row.id,
        database_id: row.database_id,
        branch_id: row.branch_id,
        organization_id: row.organization_id,
    })))
}

#[utoipa::path(
    delete,
    path = "/api/organization/{organization_id}/databases/serverless/{database_id}/branches/{branch_id}",
    params(
        ("organization_id" = Uuid, Path, description = "Organization ID"),
        ("database_id" = Uuid, Path, description = "Database ID"),
        ("branch_id" = Uuid, Path, description = "Branch ID"),
    ),
    responses(
        (status = 200, description = "Database branch link deleted"),
        (status = 404, description = "Not found"),
    ),
    tag = "databases/serverless",
)]
pub async fn delete_database_branch(
    AuthContext { tenant_db, .. }: AuthContext,
    Path((organization_id, database_id, branch_id)): Path<(Uuid, Uuid, Uuid)>,
) -> Result<Json<serde_json::Value>, AppError> {
    verify_org_access(&tenant_db, organization_id)?;

    let scoped = tenant_db.begin_scoped_transaction().await?;
    let tx = scoped.connection();

    let db = serverless_postgres_database::Entity::find_by_id(database_id)
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Serverless database not found".into()))?;

    verify_project_in_org(tx, db.project_id, organization_id).await?;

    let db_branch = serverless_postgres_database_branch::Entity::find()
        .filter(serverless_postgres_database_branch::Column::DatabaseId.eq(database_id))
        .filter(serverless_postgres_database_branch::Column::BranchId.eq(branch_id))
        .one(tx)
        .await?
        .ok_or_else(|| AppError::NotFound("Database branch link not found".into()))?;

    if db.default_branch_id == Some(db_branch.id) {
        let mut db_active: serverless_postgres_database::ActiveModel = db.into();
        db_active.default_branch_id = Set(None);
        db_active.update(tx).await?;
    }

    serverless_postgres_database_branch::Entity::delete_by_id(db_branch.id).exec(tx).await?;

    scoped.commit().await?;

    Ok(Json(serde_json::json!({ "success": true })))
}

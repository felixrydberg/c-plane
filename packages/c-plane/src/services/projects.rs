use actix_web::{HttpResponse, Result, delete, get, post, put, web};
use sea_orm::{DatabaseConnection, EntityTrait, PaginatorTrait, QueryOrder};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::errors::{AppError, ProjectError};
use crate::models::entities::project::{Entity as Project, Model as ProjectModel};
use crate::utils::logger::Logger;
use crate::utils::pagination::{PaginatedResponse, PaginationQuery};

#[derive(Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub name: String,
    pub description: Option<String>,
    pub slug: String,
    pub organisation_id: Uuid,
    pub owner_id: Uuid,
    pub is_public: bool,
}

#[derive(Serialize, Deserialize)]
pub struct ProjectResponse {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub organisation_id: Uuid,
    pub owner_id: Uuid,
    pub is_archived: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl From<ProjectModel> for ProjectResponse {
    fn from(project: ProjectModel) -> Self {
        Self {
            id: project.id,
            name: project.name,
            description: project.description,
            organisation_id: project.organisation_id,
            owner_id: project.owner_id,
            is_archived: project.is_archived,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}

#[derive(Serialize)]
pub struct ProjectsListResponse {
    pub projects: Vec<ProjectResponse>,
    pub total: u64,
    pub page: u64,
    pub per_page: u64,
}

#[get("/projects")]
pub async fn list_projects(
    db: web::Data<DatabaseConnection>,
    query: web::Query<PaginationQuery>,
) -> Result<HttpResponse, AppError> {
    let page = query.page();
    let per_page = query.per_page();

    Logger::debug(&format!(
        "Fetching projects: page={}, per_page={}",
        page, per_page
    ));

    let start_time = std::time::Instant::now();
    let paginator = Project::find()
        .order_by_desc(crate::models::entities::project::Column::CreatedAt)
        .paginate(&**db, per_page);

    let total = paginator.num_items().await?;
    let projects = paginator.fetch_page(page - 1).await?;
    let duration = start_time.elapsed();

    Logger::database_operation("SELECT", "projects", duration.as_millis() as u64);

    let project_responses: Vec<ProjectResponse> =
        projects.into_iter().map(ProjectResponse::from).collect();

    Logger::info(&format!(
        "Listed {} projects (total: {})",
        project_responses.len(),
        total
    ));

    let response = PaginatedResponse::new(project_responses, total, page, per_page);

    Ok(HttpResponse::Ok().json(response))
}

#[get("/projects/{id}")]
pub async fn get_project(
    db: web::Data<DatabaseConnection>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let project_id = path.into_inner();

    let project = Project::find_by_id(project_id)
        .one(&**db)
        .await?
        .ok_or_else(|| AppError::Project(ProjectError::ProjectNotFound(project_id)))?;

    Ok(HttpResponse::Ok().json(ProjectResponse::from(project)))
}

#[post("/projects")]
pub async fn create_project(
    db: web::Data<DatabaseConnection>,
    request: web::Json<CreateProjectRequest>,
) -> Result<HttpResponse, AppError> {
    use crate::models::entities::project::ActiveModel;
    use sea_orm::{ActiveModelTrait, Set};

    Logger::info(&format!(
        "Creating new project: name='{}', slug='{}'",
        request.name, request.slug
    ));

    let now = chrono::Utc::now().naive_utc();

    let new_project = ActiveModel {
        id: Set(Uuid::new_v4()),
        name: Set(request.name.clone()),
        description: Set(request.description.clone()),
        organisation_id: Set(request.organisation_id),
        owner_id: Set(request.owner_id),
        is_archived: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
    };

    let project = new_project.insert(&**db).await?;
    Ok(HttpResponse::Created().json(ProjectResponse::from(project)))
}

#[put("/projects/{id}")]
pub async fn update_project(
    db: web::Data<DatabaseConnection>,
    path: web::Path<Uuid>,
    request: web::Json<UpdateProjectRequest>,
) -> Result<HttpResponse, AppError> {
    use crate::models::entities::project::ActiveModel;
    use sea_orm::{ActiveModelTrait, Set};

    let project_id = path.into_inner();

    let existing_project = Project::find_by_id(project_id)
        .one(&**db)
        .await?
        .ok_or_else(|| AppError::Project(ProjectError::ProjectNotFound(project_id)))?;

    let mut project: ActiveModel = existing_project.into();
    if let Some(name) = &request.name {
        project.name = Set(name.clone());
    }
    if let Some(description) = &request.description {
        project.description = Set(description.clone());
    }
    if let Some(is_archived) = request.is_archived {
        project.is_archived = Set(is_archived);
    }

    project.updated_at = Set(chrono::Utc::now().naive_utc());

    let updated_project = project.update(&**db).await?;

    Ok(HttpResponse::Ok().json(ProjectResponse::from(updated_project)))
}

#[delete("/projects/{id}")]
pub async fn delete_project(
    db: web::Data<DatabaseConnection>,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    use sea_orm::EntityTrait;

    let project_id = path.into_inner();

    let result = Project::delete_by_id(project_id).exec(&**db).await?;

    if result.rows_affected == 0 {
        return Err(AppError::Project(ProjectError::ProjectNotFound(project_id)));
    }

    Ok(HttpResponse::NoContent().finish())
}

#[derive(Deserialize)]
pub struct UpdateProjectRequest {
    pub name: Option<String>,
    pub description: Option<Option<String>>,
    pub is_archived: Option<bool>,
}

use std::fmt;
use uuid::Uuid;

#[derive(Debug)]
pub enum ProjectError {
    InvalidSlug(String),
    SlugAlreadyExists(String),
    ProjectNotFound(Uuid),
    CannotArchivePublicProject,
    OwnerCannotLeaveProject,
    ProjectLimitExceeded { current: u32, limit: u32 },
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectError::InvalidSlug(slug) => write!(f, "Invalid slug: {}", slug),
            ProjectError::SlugAlreadyExists(slug) => write!(f, "Slug already exists: {}", slug),
            ProjectError::ProjectNotFound(id) => write!(f, "Project not found: {}", id),
            ProjectError::CannotArchivePublicProject => write!(f, "Cannot archive public project"),
            ProjectError::OwnerCannotLeaveProject => {
                write!(f, "Project owner cannot leave project")
            }
            ProjectError::ProjectLimitExceeded { current, limit } => {
                write!(f, "Project limit exceeded: {}/{}", current, limit)
            }
        }
    }
}

impl std::error::Error for ProjectError {}

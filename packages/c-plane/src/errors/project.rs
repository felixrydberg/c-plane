use std::fmt;

#[derive(Debug)]
pub enum ProjectError {
    InvalidSlug(String),
}

impl fmt::Display for ProjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProjectError::InvalidSlug(slug) => write!(f, "Invalid slug: {}", slug),
        }
    }
}

impl std::error::Error for ProjectError {}

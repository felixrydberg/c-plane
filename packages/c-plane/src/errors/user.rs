use std::fmt;
use uuid::Uuid;

#[derive(Debug)]
pub enum UserError {
    UserNotFound(Uuid),
    EmailAlreadyExists(String),
    AccountDeactivated,
    InsufficientPermissions,
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UserError::UserNotFound(id) => write!(f, "User not found: {}", id),
            UserError::EmailAlreadyExists(email) => write!(f, "Email already exists: {}", email),
            UserError::AccountDeactivated => write!(f, "Account is deactivated"),
            UserError::InsufficientPermissions => write!(f, "Insufficient permissions"),
        }
    }
}

impl std::error::Error for UserError {}

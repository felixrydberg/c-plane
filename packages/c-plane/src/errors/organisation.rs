use std::fmt;
use uuid::Uuid;

#[derive(Debug)]
pub enum OrganisationError {
    OrganisationNotFound(Uuid),
    UserNotMember(Uuid),
    InsufficientRole { required: String, current: String },
    CannotRemoveLastOwner,
}

impl fmt::Display for OrganisationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrganisationError::OrganisationNotFound(id) => {
                write!(f, "Organisation not found: {}", id)
            }
            OrganisationError::UserNotMember(id) => write!(f, "User {} is not a member", id),
            OrganisationError::InsufficientRole { required, current } => {
                write!(
                    f,
                    "Insufficient role: required {}, current {}",
                    required, current
                )
            }
            OrganisationError::CannotRemoveLastOwner => write!(f, "Cannot remove the last owner"),
        }
    }
}

impl std::error::Error for OrganisationError {}

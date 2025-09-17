use std::fmt;

#[derive(Debug)]
pub enum ExternalError {
    OryApiError(String),
    EmailServiceError(String),
    PaymentProviderError(String),
    NetworkTimeout,
    ServiceUnavailable(String),
}

impl fmt::Display for ExternalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExternalError::OryApiError(msg) => write!(f, "Ory API error: {}", msg),
            ExternalError::EmailServiceError(msg) => write!(f, "Email service error: {}", msg),
            ExternalError::PaymentProviderError(msg) => {
                write!(f, "Payment provider error: {}", msg)
            }
            ExternalError::NetworkTimeout => write!(f, "Network timeout"),
            ExternalError::ServiceUnavailable(service) => {
                write!(f, "Service unavailable: {}", service)
            }
        }
    }
}

impl std::error::Error for ExternalError {}

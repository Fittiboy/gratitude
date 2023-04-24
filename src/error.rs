use std::fmt;

use crate::error;
use crate::verification;

#[derive(Debug, thiserror::Error)]
pub enum General {
    #[error("Environment variable '{0}' not found.")]
    EnvironmentVariableNotFound(String),

    #[error("Header '{0}' not found.")]
    HeaderNotFound(String),

    #[error("Failed to deserialize from or serialize to JSON.")]
    JsonFailed(#[from] serde_json::Error),

    #[error("Invalid payload provided: {0}.")]
    InvalidPayload(String),

    #[error("Verification failed.")]
    VerificationFailed(verification::Error),

    #[error("Worker error: {0}.")]
    Worker(#[from] worker::Error),
}

#[derive(Debug)]
pub struct Http {
    pub status: Status,
    pub reason: General,
}

#[derive(Debug)]
pub enum Status {
    BadRequest = 400,
    Unauthorized = 401,
    InternalServerError = 500,
}

impl fmt::Display for Http {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "An HTTP error occurred: {}", self.reason)
    }
}

impl From<General> for Http {
    fn from(error: General) -> Self {
        Self {
            status: match &error {
                error::General::HeaderNotFound(_)
                | error::General::JsonFailed(_)
                | error::General::InvalidPayload(_) => Status::BadRequest,
                error::General::VerificationFailed(_) => Status::Unauthorized,
                _ => Status::InternalServerError,
            },
            reason: error,
        }
    }
}

impl From<worker::Error> for Http {
    fn from(error: worker::Error) -> Self {
        Self::from(error::General::from(error))
    }
}

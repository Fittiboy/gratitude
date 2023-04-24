use std::fmt;

use crate::error;

#[derive(Debug)]
pub struct Error {
    pub status: Status,
    pub reason: error::Error,
}

#[derive(Debug)]
pub enum Status {
    BadRequest = 400,
    Unauthorized = 401,
    InternalServerError = 500,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "An HTTP error occurred: {}", self.reason)
    }
}

impl From<error::Error> for Error {
    fn from(error: error::Error) -> Error {
        Error {
            status: match &error {
                error::Error::HeaderNotFound(_)
                | error::Error::JsonFailed(_)
                | error::Error::InvalidPayload(_) => Status::BadRequest,
                error::Error::VerificationFailed(_) => Status::Unauthorized,
                _ => Status::InternalServerError,
            },
            reason: error,
        }
    }
}

impl From<worker::Error> for Error {
    fn from(error: worker::Error) -> Self {
        Self::from(error::Error::from(error))
    }
}

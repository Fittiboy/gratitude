use std::fmt;

use crate::error::Error;

#[derive(Debug)]
pub struct HttpError {
    pub status: HttpStatus,
    pub reason: Error,
}

#[derive(Debug)]
pub enum HttpStatus {
    BadRequest = 400,
    Unauthorized = 401,
    InternalServerError = 500,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "An HTTP error occurred: {}", self.reason)
    }
}

impl From<Error> for HttpError {
    fn from(error: Error) -> HttpError {
        HttpError {
            status: match &error {
                Error::HeaderNotFound(_) | Error::JsonFailed(_) | Error::InvalidPayload(_) => {
                    HttpStatus::BadRequest
                }
                Error::VerificationFailed(_) => HttpStatus::Unauthorized,
                _ => HttpStatus::InternalServerError,
            },
            reason: error,
        }
    }
}

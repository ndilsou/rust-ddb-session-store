use std::error;
use std::fmt;

use serde::Deserialize;
use aws_sdk_dynamodb::types::SdkError;
use serde::Serialize;

#[derive(Debug, Serialize, Deserialize)]
pub struct AppError {
    details: String,
}

impl AppError {
    pub fn new(msg: &str) -> AppError {
        AppError {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl std::error::Error for AppError {
    fn description(&self) -> &str {
        &self.details
    }
}

impl<E> From<SdkError<E>> for AppError
where
    E: error::Error,
{
    fn from(value: SdkError<E>) -> AppError {
        AppError {
            details: format!("{}", value),
        }
    }
}

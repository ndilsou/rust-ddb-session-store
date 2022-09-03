use std::error;
use std::fmt;

use aws_sdk_dynamodb::types::SdkError;

#[derive(Debug)]
pub struct Error {
    details: String,
}

impl Error {
    pub fn new(msg: &str) -> Error {
        Error {
            details: msg.to_string(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.details)
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        &self.details
    }
}
impl<E> From<SdkError<E>> for Error
where
    E: error::Error,
{
    fn from(value: SdkError<E>) -> Error {
        Error {
            details: format!("{}", value),
        }
    }
}

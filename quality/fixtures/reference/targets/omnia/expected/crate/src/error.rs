use omnia_guest::Error as GuestError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ResetError {
    #[error("invalid email")]
    InvalidEmail,
    #[error("publish failed: {0}")]
    PublishUnavailable(String),
}

impl ResetError {
    pub fn code(&self) -> &'static str {
        match self {
            ResetError::InvalidEmail => "invalid_email",
            ResetError::PublishUnavailable(_) => "publish_unavailable",
        }
    }
}

impl From<ResetError> for GuestError {
    fn from(err: ResetError) -> Self {
        match &err {
            ResetError::InvalidEmail => GuestError::BadRequest {
                code: err.code().to_string(),
                description: err.to_string(),
            },
            ResetError::PublishUnavailable(_) => GuestError::BadGateway {
                code: err.code().to_string(),
                description: err.to_string(),
            },
        }
    }
}

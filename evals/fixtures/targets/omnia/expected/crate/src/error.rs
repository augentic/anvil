use omnia_sdk::Error as SdkError;
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

impl From<ResetError> for SdkError {
    fn from(err: ResetError) -> Self {
        match &err {
            ResetError::InvalidEmail => SdkError::BadRequest {
                code: err.code().to_string(),
                description: err.to_string(),
            },
            ResetError::PublishUnavailable(_) => SdkError::BadGateway {
                code: err.code().to_string(),
                description: err.to_string(),
            },
        }
    }
}

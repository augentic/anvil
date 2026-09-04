//! DTOs mirroring the `emery:adapter` WIT records.
//!
//! Only answer-deserialized types carry serde derives.

mod source;

pub use source::{
    Authority, Backing, Claim, ClaimKind, Evidence, SourceContent, SourceInput, SourceMetadata,
    SourceWorkspace,
};

/// Adapter operation error.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// Malformed request.
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// Filesystem failure.
    #[error("io: {0}")]
    Io(String),
    /// Judgment or answer-handling failure.
    #[error("internal: {0}")]
    Internal(String),
}

impl From<omnia_guest::model::Error> for Error {
    fn from(err: omnia_guest::model::Error) -> Self {
        match err {
            omnia_guest::model::Error::InvalidRequest(detail) => Self::InvalidRequest(detail),
            other => Self::Internal(other.to_string()),
        }
    }
}

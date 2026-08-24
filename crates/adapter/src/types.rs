//! DTOs mirroring the `emery:adapter` WIT records.
//!
//! Only answer-deserialized types carry serde derives.

use std::path::Path;

use crate::registry::Doc;

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

/// Call-scoped adapter environment.
#[derive(Clone, Debug)]
pub struct Context<'a> {
    /// Routed adapter identity.
    pub adapter_id: &'a str,
    /// Guest `"."` preopen root.
    pub project_root: &'a Path,
    /// Embedded reference documents served by the judgment's tool closure.
    pub docs: &'static [Doc],
    /// Workspace lend, absent for inline values.
    pub lend: Option<String>,
}

impl<'a> Context<'a> {
    /// Creates `"."` guest context.
    #[must_use]
    pub fn guest(adapter_id: &'a str) -> Self {
        Self {
            adapter_id,
            project_root: Path::new("."),
            docs: &[],
            lend: Some(".".to_string()),
        }
    }

    /// Replaces the reference corpus the judgment's tool closure serves.
    #[must_use]
    pub const fn with_docs(mut self, docs: &'static [Doc]) -> Self {
        self.docs = docs;
        self
    }

    /// Replaces the judgment workspace lend with `path`.
    #[must_use]
    pub fn lending(mut self, path: impl Into<String>) -> Self {
        self.lend = Some(path.into());
        self
    }

    /// Removes the workspace lend for an inline value.
    #[must_use]
    pub fn without_lend(mut self) -> Self {
        self.lend = None;
        self
    }
}

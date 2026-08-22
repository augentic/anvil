//! DTOs mirroring the `emery:adapter` WIT records.
//!
//! Only answer-deserialized types carry serde derives.

use std::path::Path;

use omnia_guest::model::McpGrant;

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
    /// Runtime-injected MCP references endpoint.
    pub mcp_url: Option<String>,
    /// Workspace lend, absent for inline values.
    pub lend: Option<String>,
}

impl<'a> Context<'a> {
    /// Creates `"."` guest context with an injected references URL.
    #[must_use]
    pub fn guest(adapter_id: &'a str) -> Self {
        Self {
            adapter_id,
            project_root: Path::new("."),
            mcp_url: mcp_url(adapter_id),
            lend: Some(".".to_string()),
        }
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

    /// Returns the references grant, named without axis or version.
    #[must_use]
    pub fn grants(&self) -> Vec<McpGrant> {
        let name = self.adapter_id.rsplit(':').next().unwrap_or(self.adapter_id);
        let name = name.split_once('@').map_or(name, |(stem, _)| stem);
        self.mcp_url
            .as_deref()
            .map(|url| McpGrant {
                name: format!("{name}-references"),
                tools: Vec::new(),
                url: url.to_string(),
            })
            .into_iter()
            .collect()
    }
}

/// Derives the references URL from `MCP_URL_BASE`, then `HTTP_ADDR`.
#[must_use]
#[expect(
    clippy::disallowed_methods,
    reason = "the seam derives its shelf URL from the runtime-injected listener env; \
              this is engine infrastructure, not app configuration"
)]
pub fn mcp_url(adapter_id: &str) -> Option<String> {
    if let Ok(base) = std::env::var("MCP_URL_BASE") {
        return mcp_url_with_base(&base, adapter_id);
    }
    mcp_url_for(std::env::var("HTTP_ADDR").ok().as_deref(), adapter_id)
}

/// Builds from `base`, replacing the axis separator and preserving the pin.
#[must_use]
pub fn mcp_url_with_base(base: &str, adapter_id: &str) -> Option<String> {
    let base = base.trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    Some(format!("{base}/mcp/{}", adapter_id.replacen(':', "/", 1)))
}

/// Builds a loopback references URL from an `HTTP_ADDR`.
#[must_use]
pub fn mcp_url_for(addr: Option<&str>, adapter_id: &str) -> Option<String> {
    let port = addr?.rsplit_once(':')?.1.parse::<u16>().ok()?;
    mcp_url_with_base(&format!("http://127.0.0.1:{port}"), adapter_id)
}

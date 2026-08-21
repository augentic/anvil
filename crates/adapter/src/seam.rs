//! Seam vocabulary mirroring the `emery:adapter` WIT records.
//!
//! Only answer-deserialized types carry serde derives.

use std::path::Path;

use omnia_guest::model::McpGrant;

mod source;

pub use source::{
    Authority, Backing, Claim, ClaimKind, Evidence, SourceContent, SourceInput, SourceMetadata,
    SourceWorkspace,
};

/// Operation error — mirrors the WIT `types.error` variant.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    /// The request itself is malformed.
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    /// A filesystem operation failed.
    #[error("io: {0}")]
    Io(String),
    /// A judgment call or answer-handling step failed.
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

/// Call-scoped environment the shim resolves and hands to every operation.
#[derive(Clone, Debug)]
pub struct Context<'a> {
    /// Adapter identity this call was routed by, e.g. `target:contracts`.
    pub adapter_id: &'a str,
    /// The guest's `"."` preopen root (the shared project mount).
    pub project_root: &'a Path,
    /// Adapter MCP references endpoint, when the runtime injected one.
    pub mcp_url: Option<String>,
    /// Judgment-leg workspace lend; `None` for an inline value.
    pub lend: Option<String>,
}

impl<'a> Context<'a> {
    /// Guest context at `"."`, granting [`mcp_url`] when injected.
    #[must_use]
    pub fn guest(adapter_id: &'a str) -> Self {
        Self {
            adapter_id,
            project_root: Path::new("."),
            mcp_url: mcp_url(adapter_id),
            lend: Some(".".to_string()),
        }
    }

    /// Lend `path` to judgment legs instead of the `"."` project mount.
    #[must_use]
    pub fn lending(mut self, path: impl Into<String>) -> Self {
        self.lend = Some(path.into());
        self
    }

    /// Issue the judgment call with no workspace lend (inline `value`).
    #[must_use]
    pub fn without_lend(mut self) -> Self {
        self.lend = None;
        self
    }

    /// Adapter references grant, named from the axis- and version-stripped id.
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

/// Adapter references URL: `MCP_URL_BASE` or `HTTP_ADDR`, else `None`.
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

/// [`mcp_url`] over an injected base (`:` → `/`; the version pin stays).
#[must_use]
pub fn mcp_url_with_base(base: &str, adapter_id: &str) -> Option<String> {
    let base = base.trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    Some(format!("{base}/mcp/{}", adapter_id.replacen(':', "/", 1)))
}

/// [`mcp_url`] over `HTTP_ADDR`; host is `127.0.0.1`, never `localhost`.
#[must_use]
pub fn mcp_url_for(addr: Option<&str>, adapter_id: &str) -> Option<String> {
    let port = addr?.rsplit_once(':')?.1.parse::<u16>().ok()?;
    mcp_url_with_base(&format!("http://127.0.0.1:{port}"), adapter_id)
}

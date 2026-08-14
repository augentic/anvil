//! Locator grammar shared by source and target rows.

use std::path::{Path, PathBuf};

use error::Error;

/// One location-backed binding: a locator plus an optional path selector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Location {
    /// Git (`url@revision`), filesystem path, or bounded HTTPS URL.
    pub locator: Locator,
    /// Subtree or file inside the locator root. Default `.`.
    pub path: String,
}

/// Closed locator forms (RFC-88 D2).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Locator {
    /// Git working tree at an exact or mutable revision.
    Git {
        /// Credential-free clone URL, `file://` path, or local path.
        url: String,
        /// Branch, tag, or commit. Mutable refs become exact SHAs at ingest.
        revision: String,
    },
    /// Change-relative or external local filesystem path.
    Path(PathBuf),
    /// Bounded HTTPS document URL (not a Git clone).
    Https(String),
}

/// Shared row arm: location-backed locator or inline value.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Origin {
    /// Location-backed source or target.
    Location(Location),
    /// Inline value protected by the plan digest; no CID.
    Value(String),
}

impl Location {
    /// Parse `raw` and apply `path` (default `.`).
    ///
    /// # Errors
    ///
    /// `locator-malformed`, `locator-http-unsupported`,
    /// `locator-credentials-forbidden`.
    pub fn parse(raw: &str, path: Option<&str>) -> Result<Self, Error> {
        let locator = Locator::parse(raw)?;
        let path = path.unwrap_or(".").trim();
        let path = if path.is_empty() { "." } else { path };
        if Path::new(path).is_absolute() {
            return Err(malformed("path selector must be relative"));
        }
        Ok(Self {
            locator,
            path: path.to_string(),
        })
    }

    /// Intern key: canonical locator plus path selector.
    #[must_use]
    pub fn key(&self) -> String {
        format!("{}#{}", self.locator.key(), self.path)
    }
}

impl Locator {
    /// Parse a locator string.
    ///
    /// Git is `url@revision`. `https://` without a Git suffix is a document
    /// URL. Everything else is a filesystem path.
    ///
    /// # Errors
    ///
    /// `locator-malformed`, `locator-http-unsupported`,
    /// `locator-credentials-forbidden`.
    pub fn parse(raw: &str) -> Result<Self, Error> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(malformed("locator is empty"));
        }
        if raw.starts_with("http://") {
            return Err(Error::Diag {
                code: "locator-http-unsupported",
                detail: "remote locators require HTTPS".into(),
            });
        }
        if let Some((url, revision)) = git_pair(raw) {
            refuse_userinfo(url)?;
            return Ok(Self::Git {
                url: canon_git_url(url),
                revision: revision.to_string(),
            });
        }
        if raw.starts_with("https://") {
            refuse_userinfo(raw)?;
            return Ok(Self::Https(raw.to_string()));
        }
        Ok(Self::Path(PathBuf::from(raw)))
    }

    /// Canonical intern / display form.
    #[must_use]
    pub fn key(&self) -> String {
        match self {
            Self::Git { url, revision } => format!("{url}@{revision}"),
            Self::Path(path) => path.display().to_string(),
            Self::Https(url) => url.clone(),
        }
    }

    /// Whether `revision` is already an exact commit id.
    #[must_use]
    pub fn is_sha(revision: &str) -> bool {
        let n = revision.len();
        (n == 40 || n == 64) && revision.bytes().all(|b| b.is_ascii_hexdigit())
    }
}

impl Origin {
    /// Location-backed origin.
    #[must_use]
    pub const fn location(location: Location) -> Self {
        Self::Location(location)
    }

    /// Inline value origin.
    #[must_use]
    pub const fn value(value: String) -> Self {
        Self::Value(value)
    }
}

fn git_pair(raw: &str) -> Option<(&str, &str)> {
    let (url, revision) = raw.rsplit_once('@')?;
    if revision.is_empty() || revision.contains(['?', '#', '@']) {
        return None;
    }
    is_git_url(url).then_some((url, revision))
}

fn is_git_url(url: &str) -> bool {
    if let Some(rest) = url.strip_prefix("https://") {
        let host = rest.split('/').next().unwrap_or("");
        return !host.is_empty() && rest.contains('/') && !host.contains('@');
    }
    if url.starts_with("file://") || url.starts_with("ssh://") || url.starts_with("git@") {
        return true;
    }
    let path = Path::new(url);
    path.is_absolute() || url.starts_with("./") || url.starts_with("../")
}

fn refuse_userinfo(url: &str) -> Result<(), Error> {
    let Some(rest) = url.strip_prefix("https://") else {
        return Ok(());
    };
    let host = rest.split('/').next().unwrap_or("");
    if host.contains('@') {
        return Err(Error::Diag {
            code: "locator-credentials-forbidden",
            detail: "remote locators must not contain credentials".into(),
        });
    }
    Ok(())
}

fn canon_git_url(url: &str) -> String {
    let trimmed = url.trim_end_matches('/');
    trimmed.strip_suffix(".git").unwrap_or(trimmed).to_string()
}

fn malformed(detail: &str) -> Error {
    Error::Diag {
        code: "locator-malformed",
        detail: detail.into(),
    }
}

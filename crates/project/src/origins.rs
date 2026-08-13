//! Origin-locator classification (RFC-104) plus the native fetch
//! kernel (host `git` / HTTPS download); the engine guest fetches
//! through the host-implemented `emery:origins` import instead.

#[cfg(not(target_arch = "wasm32"))]
mod fetch;

#[cfg(not(target_arch = "wasm32"))]
pub use fetch::{FetchedTree, discard, fetch};

/// True when `location` is a remote origin locator rather than a
/// local path.
#[must_use]
pub fn is_remote(location: &str) -> bool {
    ["https://", "http://", "ssh://", "git@"].iter().any(|prefix| location.starts_with(prefix))
}

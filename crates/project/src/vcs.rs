//! Origin-locator classification plus the native VCS kernels
//! (RFC-95 `emery:vcs`: tree fetch and D11 worktree export), run
//! in-process natively; the engine guest imports `emery:vcs`.

#[cfg(not(target_arch = "wasm32"))]
pub mod forge;
#[cfg(not(target_arch = "wasm32"))]
mod host;
#[cfg(not(target_arch = "wasm32"))]
pub mod worktree;

#[cfg(not(target_arch = "wasm32"))]
pub use host::{FetchedTree, discard, fetch, sweep_stale};

/// True when `location` is a remote origin locator rather than a
/// local path.
#[must_use]
pub fn is_remote(location: &str) -> bool {
    ["https://", "http://", "ssh://", "git@"].iter().any(|prefix| location.starts_with(prefix))
}

//! Test-only helpers for pinning the out-of-tree project cache.
//!
//! The manifest mirror and codex pack live in a per-project OS cache
//! resolved by `specify_schema::cache`. Unit tests that drive
//! `init`/`sync` and then assert on cache contents must redirect that
//! cache into a temp directory so the writes are hermetic and never
//! touch the developer's `~/.cache`.

use std::ffi::OsString;
use std::path::Path;

use tempfile::TempDir;

const CACHE_ENV: &str = "SPECIFY_PROJECT_CACHE";

/// Restores the previous `SPECIFY_PROJECT_CACHE` value on drop.
pub struct CacheGuard(Option<OsString>);

impl Drop for CacheGuard {
    #[expect(unsafe_code, reason = "restore the cache-root env var pinned for the test")]
    fn drop(&mut self) {
        // SAFETY: nextest runs each test in its own process, so no other
        // thread observes the env mutation for the guard's lifetime.
        unsafe {
            match self.0.take() {
                Some(prev) => std::env::set_var(CACHE_ENV, prev),
                None => std::env::remove_var(CACHE_ENV),
            }
        }
    }
}

/// Pin the out-of-tree cache root inside `tmp` for the test's lifetime
/// so cache writes are hermetic and auto-cleaned with the tempdir.
#[expect(unsafe_code, reason = "pin the cache-root env var into the test tempdir")]
pub fn scoped_cache(tmp: &TempDir) -> CacheGuard {
    let prev = std::env::var_os(CACHE_ENV);
    // SAFETY: see `CacheGuard::drop` — single-process test isolation.
    unsafe { std::env::set_var(CACHE_ENV, tmp.path().join("project-cache")) };
    CacheGuard(prev)
}

/// The out-of-tree cache directory for `project_dir` under the pinned
/// root. Mirror of the production resolver, for assertions.
pub fn expected_cache_dir(project_dir: &Path) -> std::path::PathBuf {
    specify_schema::cache::project_cache_dir(project_dir)
}

const STORE_ENV: &str = "SPECIFY_ADAPTER_CACHE";

/// Restores the previous `SPECIFY_ADAPTER_CACHE` value on drop.
pub struct StoreGuard(Option<OsString>);

impl Drop for StoreGuard {
    #[expect(unsafe_code, reason = "restore the store-root env var pinned for the test")]
    fn drop(&mut self) {
        // SAFETY: nextest runs each test in its own process, so no other
        // thread observes the env mutation for the guard's lifetime.
        unsafe {
            match self.0.take() {
                Some(prev) => std::env::set_var(STORE_ENV, prev),
                None => std::env::remove_var(STORE_ENV),
            }
        }
    }
}

/// Pin the global adapter store root at `root` for the test's lifetime so
/// store reads resolve into a hermetic temp directory (RFC-48 D5).
#[expect(unsafe_code, reason = "pin the store-root env var into the test tempdir")]
pub fn scoped_store(root: &Path) -> StoreGuard {
    let prev = std::env::var_os(STORE_ENV);
    // SAFETY: see `StoreGuard::drop` — single-process test isolation.
    unsafe { std::env::set_var(STORE_ENV, root) };
    StoreGuard(prev)
}

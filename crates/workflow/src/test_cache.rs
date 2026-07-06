//! Test-only helpers for pinning the out-of-tree adapter store.
//!
//! The global adapter store lives in an OS cache resolved by
//! `specify_schema::cache`. Tests that drive adapter install/resolve
//! must redirect that store into a temp directory so reads are
//! hermetic and never touch the developer's real store.

use std::ffi::OsString;
use std::path::Path;

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

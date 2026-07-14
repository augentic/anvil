//! Environment guards pinning the out-of-tree cache and store roots
//! inside test tempdirs, plus the dev-build staging helper.

use std::path::{Path, PathBuf};

const CACHE_ENV: &str = "SPECIFY_PROJECT_CACHE";

/// Restores the previous `SPECIFY_PROJECT_CACHE` value on drop.
#[derive(Debug)]
pub struct CacheGuard(Option<std::ffi::OsString>);

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

/// Pin the out-of-tree project cache root inside `dir` so adapter
/// cache writes are hermetic and auto-cleaned with the tempdir.
#[must_use]
#[expect(unsafe_code, reason = "pin the cache-root env var into the test tempdir")]
pub fn scoped_cache(dir: &Path) -> CacheGuard {
    let prev = std::env::var_os(CACHE_ENV);
    // SAFETY: see `CacheGuard::drop` — single-process test isolation.
    unsafe { std::env::set_var(CACHE_ENV, dir.join("project-cache")) };
    CacheGuard(prev)
}

/// Out-of-tree cache directory for `project_dir` under the pinned root.
#[must_use]
pub fn expected_cache_dir(project_dir: &Path) -> PathBuf {
    diagnostics::cache::project_cache_dir(project_dir)
}

const STORE_ENV: &str = "SPECIFY_ADAPTER_STORE";

/// Restores the previous `SPECIFY_ADAPTER_STORE` value on drop.
#[derive(Debug)]
pub struct StoreGuard(Option<std::ffi::OsString>);

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

/// Pin the global content-addressed adapter store root
/// directly at `dir` so install / resolve probes are hermetic and
/// auto-cleaned with the tempdir.
#[must_use]
#[expect(unsafe_code, reason = "pin the store-root env var into the test tempdir")]
pub fn scoped_store(dir: &Path) -> StoreGuard {
    let prev = std::env::var_os(STORE_ENV);
    // SAFETY: see `StoreGuard::drop` — single-process test isolation.
    unsafe { std::env::set_var(STORE_ENV, dir) };
    StoreGuard(prev)
}

/// Stage a stub adapter component for `name` at the resolver's in-repo
/// development probe.
///
/// The stub lands at `<root>/target/wasm32-wasip2/release/<name>.wasm`,
/// so a bare-name resolve inside `root` can dispatch the test metadata
/// runner.
///
/// # Panics
///
/// Panics when the dev release directory or the stub file cannot be
/// written.
pub fn stage_dev_component(root: &Path, name: &str) {
    let dev_dir = root.join("target/wasm32-wasip2/release");
    std::fs::create_dir_all(&dev_dir).expect("mkdir dev release dir");
    std::fs::write(dev_dir.join(format!("{}.wasm", name.replace('-', "_"))), "{}")
        .expect("write stub component");
}

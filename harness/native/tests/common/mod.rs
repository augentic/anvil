//! Shared support for the native harness suites: a throw-away project
//! tree with a hermetic project cache.

use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

const CACHE_ENV: &str = "SPECIFY_PROJECT_CACHE";

/// Restores the previous `SPECIFY_PROJECT_CACHE` value on drop.
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

/// An initialised throw-away project bound to the `omnia` target, with
/// the adapter cache pinned inside the tempdir.
pub struct Project {
    _tmp: TempDir,
    _cache: CacheGuard,
    root: PathBuf,
}

impl Project {
    /// Scaffold the tree: `.specify/{slices,specs}`, `project.yaml`
    /// bound to the linked `omnia` adapter.
    #[expect(unsafe_code, reason = "pin the cache-root env var into the test tempdir")]
    pub fn new() -> Self {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().canonicalize().expect("canonical tempdir");
        let prev = std::env::var_os(CACHE_ENV);
        // SAFETY: see `CacheGuard::drop` — single-process test isolation.
        unsafe { std::env::set_var(CACHE_ENV, root.join("project-cache")) };
        for sub in [".specify/slices", ".specify/specs"] {
            fs::create_dir_all(root.join(sub)).expect("mkdir");
        }
        fs::write(root.join(".specify/project.yaml"), "name: demo\nadapter: omnia\nrules: {}\n")
            .expect("write project.yaml");
        Self {
            _tmp: tmp,
            _cache: CacheGuard(prev),
            root,
        }
    }

    /// The project root.
    #[must_use]
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }
}

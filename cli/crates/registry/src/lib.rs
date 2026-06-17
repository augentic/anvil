//! Specify's declared WASI tool model, cache, resolver, and
//! Wasmtime-backed execution host. See `DECISIONS.md`
//! §"Extension architecture" for the canonical contract.

pub mod cache;
pub mod error;
pub mod host;
pub mod load;
pub mod oci;
pub mod pack;
mod package;
pub mod permissions;
pub mod resolver;
pub mod store;
// The manifest DTOs and structural validation live in the wasmtime-free
// `specify-extension-manifest` leaf; re-export them under their historical
// module paths so `specify_registry::manifest::Extension` and
// `specify_registry::validate` keep resolving.
pub use package::PackageMetadata;
pub use specify_extension_manifest as manifest;
pub use specify_extension_manifest::{DEFAULT_WASM_PKG_CONFIG, WASM_PKG_CONFIG_FILENAME, validate};

#[cfg(test)]
#[expect(unsafe_code, reason = "test helpers mutate process-wide env vars under env_lock")]
mod test_support {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::{env, fs};

    use jiff::Timestamp;

    use crate::cache;
    use crate::manifest::{Axis, Extension, ExtensionPermissions, ExtensionScope, ExtensionSource};

    static SCRATCH_COUNTER: AtomicU64 = AtomicU64::new(0);

    pub fn fixed_now() -> Timestamp {
        "2026-05-07T00:00:00Z".parse().expect("fixed test stamp")
    }

    pub fn project_scope() -> ExtensionScope {
        ExtensionScope::Project {
            project_name: "demo".to_string(),
        }
    }

    pub fn plugin_target_scope(root: &Path) -> ExtensionScope {
        ExtensionScope::Plugin {
            axis: Axis::Target,
            plugin_slug: "contracts".to_string(),
            capability_dir: root.to_path_buf(),
        }
    }

    pub fn tool(source: ExtensionSource, sha256: Option<String>) -> Extension {
        Extension {
            name: "contract".to_string(),
            version: "1.0.0".to_string(),
            source,
            sha256,
            permissions: ExtensionPermissions::default(),
        }
    }

    pub fn named_tool(name: &str, source: ExtensionSource, sha256: Option<String>) -> Extension {
        Extension {
            name: name.to_string(),
            ..tool(source, sha256)
        }
    }

    pub fn write_source(root: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = root.join(name);
        fs::write(&path, bytes).expect("write source");
        path
    }

    pub fn cached_bytes(scope: &ExtensionScope, tool: &Extension) -> Vec<u8> {
        fs::read(cache::module_path(scope, &tool.name, &tool.version).expect("module path"))
            .expect("read cached module")
    }

    /// Lock guarding process-wide environment mutations in tests.
    pub fn env_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Acquire the env lock and pin the cache-root precedence to
    /// `SPECIFY_EXTENSIONS_CACHE = cache_dir`, ensuring `XDG_CACHE_HOME` and
    /// `HOME` cannot leak from the host. Tuple fields drop in
    /// declaration order, so the `EnvGuard` array restores env vars
    /// first and the lock guard releases last.
    pub fn cache_env(cache_dir: &Path) -> ([EnvGuard; 3], MutexGuard<'static, ()>) {
        let lock = env_lock();
        let guards = [
            EnvGuard::scoped("SPECIFY_EXTENSIONS_CACHE", Some(cache_dir)),
            EnvGuard::scoped("XDG_CACHE_HOME", None),
            EnvGuard::scoped("HOME", None),
        ];
        (guards, lock)
    }

    /// Create a unique temporary directory for tests.
    pub fn scratch_dir(label: &str) -> PathBuf {
        let n = SCRATCH_COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos =
            SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_nanos());
        let dir = env::temp_dir()
            .join(format!("specify-tool-{label}-{}-{nanos}-{n}", std::process::id()));
        fs::create_dir_all(&dir).expect("create scratch dir");
        dir
    }

    /// Snapshot a process-wide env var and restore on drop.
    ///
    /// Tests construct one of these per env var they touch, always
    /// after acquiring [`env_lock`], so concurrent tests cannot
    /// observe partial state. `Drop` runs in reverse declaration
    /// order, which keeps restores ordered relative to the lock.
    pub struct EnvGuard {
        key: &'static str,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        pub fn scoped(key: &'static str, value: Option<&Path>) -> Self {
            let previous = env::var_os(key);
            // SAFETY: callers hold `env_lock`, so no concurrent reader
            // can observe partial state.
            unsafe {
                match value {
                    Some(value) => env::set_var(key, value),
                    None => env::remove_var(key),
                }
            }
            Self { key, previous }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            // SAFETY: see [`EnvGuard::scoped`].
            unsafe {
                match self.previous.take() {
                    Some(value) => env::set_var(self.key, value),
                    None => env::remove_var(self.key),
                }
            }
        }
    }
}

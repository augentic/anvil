//! Suite-local provider and helpers for the resolver / install /
//! store suites: the shipped `resolver::Component` behind a
//! deterministic metadata runner, a file-backed hydrator, and the
//! env guards pinning the cache and store roots into tempdirs.

#![allow(dead_code, reason = "each test binary uses a subset of the shared support surface")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use error::Error;
use project::adapter::metadata::Metadata;
use project::adapter::{AdapterRef, ResolvedSource, ResolvedTarget, Resolver};
use serde_json::json;

/// The concrete provider the resolver-flavoured suites run against:
/// exactly the capabilities the init / resolve operations bind
/// (`Anchor + Resolver + Hydrator`) — no adapter catalog, no model.
#[derive(Clone)]
pub struct Provider {
    /// The project root every project-scoped verb anchors at.
    pub root: PathBuf,
    // Owned tempdir + env pinning; dropped with the last clone.
    _owned: Arc<Owned>,
}

struct Owned {
    _cache: harness::env::CacheGuard,
    _tmp: tempfile::TempDir,
}

impl Provider {
    /// A bare directory — nothing scaffolded: owned tempdir with the
    /// out-of-tree project cache pinned inside it.
    ///
    /// # Panics
    ///
    /// Panics when the tempdir cannot be created.
    #[must_use]
    pub fn bare() -> Self {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let root = tmp.path().canonicalize().expect("canonical tempdir");
        let cache = harness::env::scoped_cache(&root);
        Self {
            root,
            _owned: Arc::new(Owned {
                _cache: cache,
                _tmp: tmp,
            }),
        }
    }
}

impl project::handler::Anchor for Provider {
    fn project_root(&self) -> &Path {
        &self.root
    }
}

impl Resolver for Provider {
    fn resolve_source(
        &self, adapter_ref: &AdapterRef, project_dir: &Path,
    ) -> Result<ResolvedSource, Error> {
        Resolver::resolve_source(&resolver(), adapter_ref, project_dir)
    }

    fn resolve_target(
        &self, adapter_ref: &AdapterRef, project_dir: &Path,
    ) -> Result<ResolvedTarget, Error> {
        Resolver::resolve_target(&resolver(), adapter_ref, project_dir)
    }
}

// A file-backed registry: a test stages the expected component bytes
// at `<root>/hydrator/<name>@<version>.wasm` and the fetch serves
// them; an unstaged URL refuses, standing in for a fetch failure.
impl project::adapter::Hydrator for Provider {
    async fn fetch(&self, url: &str) -> Result<Vec<u8>, Error> {
        let entry = url.rsplit('/').next().unwrap_or_default();
        let staged = self.root.join("hydrator").join(entry);
        std::fs::read(&staged).map_err(|err| Error::Diag {
            code: "http-fetch",
            detail: format!("no staged registry response for {url}: {err}"),
        })
    }
}

/// The shipped component resolver with the deterministic
/// [`metadata_json`] answers behind its metadata runner — file probing
/// intact for the resolve / install / store suites.
#[must_use]
pub fn resolver() -> project::adapter::resolver::Component {
    fn stub(request: &project::adapter::metadata::Request<'_>) -> Result<Metadata, Error> {
        serde_json::from_str(&metadata_json(request.adapter_id)).map_err(|err| Error::Diag {
            code: "adapter-metadata-failed",
            detail: format!("fixture metadata parse {}: {err}", request.adapter_id),
        })
    }

    project::adapter::resolver::Component::new(stub)
}

/// The deterministic resolve-time metadata JSON a routed adapter id
/// answers. The special identities:
///
/// - `target:demo-target` — a `specify` floor newer than any real
///   binary (the `adapter-cli-too-old` gate);
/// - `target:bad-floor` — an unparseable floor
///   (`adapter-floor-malformed`);
/// - `target:vectis` — declared build inputs plus the full
///   three-platform capability;
/// - anything else — `{}` (no floor, no inputs, no capability).
#[must_use]
pub fn metadata_json(adapter_id: &str) -> String {
    match adapter_id {
        "target:demo-target" => r#"{"specify-floor":"999.0.0"}"#.to_string(),
        "target:bad-floor" => r#"{"specify-floor":"v1"}"#.to_string(),
        "target:vectis" => json!({
            "inputs": [
                { "path": "tokens.yaml", "required": true },
                { "path": "assets.yaml", "required": false },
            ],
            "platforms": {
                "required": true,
                "allowed": ["core", "ios", "android"],
                "default": ["core", "ios", "android"],
            },
        })
        .to_string(),
        _ => "{}".to_string(),
    }
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

/// Recursively copy `src` into `dst`, creating directories as needed.
///
/// # Panics
///
/// Panics if `src` cannot be read or a file cannot be copied.
pub fn copy_dir(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).expect("create_dir_all dst");
    for entry in std::fs::read_dir(src).expect("read_dir src") {
        let entry = entry.expect("dir entry");
        let target = dst.join(entry.file_name());
        if entry.file_type().expect("file_type").is_dir() {
            copy_dir(&entry.path(), &target);
        } else {
            std::fs::copy(entry.path(), &target).expect("copy");
        }
    }
}

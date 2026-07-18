//! Suite-local provider and helpers for the resolver / install /
//! store suites: the shipped `resolver::Component` behind a
//! deterministic metadata runner, a file-backed ensure fetch, and the
//! env guard pinning the store root into a tempdir (the project cache
//! is isolated through the provider's [`ExecutionPaths`]).

#![allow(dead_code, reason = "each test binary uses a subset of the shared support surface")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use error::Error;
use project::adapter::metadata::Metadata;
use project::adapter::{AdapterSelector, ResolvedSource, ResolvedTarget, Resolver};
use project::handler::ExecutionPaths;
use serde_json::json;

/// The concrete provider the resolver-flavoured suites run against:
/// exactly the capabilities the init / resolve operations bind
/// (`Anchor + Resolver`) — no adapter catalog, no model.
#[derive(Clone)]
#[expect(
    clippy::partial_pub_fields,
    reason = "tests read `root` directly; the tempdir is lifetime detail"
)]
pub struct Provider {
    /// The project root every project-scoped verb anchors at.
    pub root: PathBuf,
    paths: ExecutionPaths,
    // Owned tempdir; dropped with the last clone.
    _owned: Arc<tempfile::TempDir>,
}

impl Provider {
    /// A bare directory — nothing scaffolded: owned tempdir with the
    /// out-of-tree project cache isolated inside it.
    ///
    /// # Panics
    ///
    /// Panics when the tempdir cannot be created.
    #[must_use]
    pub fn bare() -> Self {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let root = tmp.path().canonicalize().expect("canonical tempdir");
        let paths = ExecutionPaths::isolated(&root, root.join("project-cache"));
        Self {
            root,
            paths,
            _owned: Arc::new(tmp),
        }
    }
}

impl project::handler::Anchor for Provider {
    fn paths(&self) -> &ExecutionPaths {
        &self.paths
    }
}

impl Resolver for Provider {
    fn resolve_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        Resolver::resolve_source(&resolver(), selector, paths)
    }

    fn resolve_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        Resolver::resolve_target(&resolver(), selector, paths)
    }

    // The component-deployment ensure kernels over a file-backed
    // registry: a test stages the expected component bytes at
    // `<root>/hydrator/<name>@<version>.wasm` and the fetch serves
    // them; an unstaged URL refuses, standing in for a fetch failure.
    async fn ensure_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        project::adapter::ensure::source(stub_metadata, selector, paths, test_now(), |url| {
            let response = staged_fetch(&self.root, &url);
            async move { response }
        })
        .await
    }

    async fn ensure_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        project::adapter::ensure::target(stub_metadata, selector, paths, test_now(), |url| {
            let response = staged_fetch(&self.root, &url);
            async move { response }
        })
        .await
    }
}

/// Deterministic timestamp for ensure provenance stamps.
const fn test_now() -> jiff::Timestamp {
    jiff::Timestamp::UNIX_EPOCH
}

/// Serve staged registry bytes from `<root>/hydrator/<entry>`.
fn staged_fetch(root: &Path, url: &str) -> Result<Vec<u8>, Error> {
    let entry = url.rsplit('/').next().unwrap_or_default();
    let staged = root.join("hydrator").join(entry);
    std::fs::read(&staged).map_err(|err| Error::Diag {
        code: "http-fetch",
        detail: format!("no staged registry response for {url}: {err}"),
    })
}

/// The deterministic metadata runner behind [`resolver`], as a plain
/// `fn` for the ensure kernels.
fn stub_metadata(request: &project::adapter::metadata::Request<'_>) -> Result<Metadata, Error> {
    serde_json::from_str(&metadata_json(request.adapter_id)).map_err(|err| Error::Diag {
        code: "adapter-metadata-failed",
        detail: format!("fixture metadata parse {}: {err}", request.adapter_id),
    })
}

/// The shipped component resolver with the deterministic
/// [`metadata_json`] answers behind its metadata runner — file probing
/// intact for the resolve / install / store suites.
#[must_use]
pub fn resolver() -> project::adapter::resolver::Component {
    project::adapter::resolver::Component::new(stub_metadata)
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

/// Out-of-tree cache directory for the provider's project root under
/// its isolated cache parent.
#[must_use]
pub fn expected_cache_dir(provider: &Provider) -> PathBuf {
    provider.paths.cache_dir()
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

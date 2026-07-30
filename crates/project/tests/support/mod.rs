//! Suite-local provider and helpers for the resolver / install /
//! store suites: the shipped `resolver::Component` behind a
//! deterministic metadata runner. The store root and project cache
//! are isolated inside the provider's tempdir through carried
//! explicit [`Locations`] — no process environment is read or
//! mutated.

#![allow(dead_code, reason = "each test binary uses a subset of the shared support surface")]

use std::path::{Path, PathBuf};
use std::sync::Arc;

use error::Error;
use project::adapter::metadata::Metadata;
use project::adapter::{AdapterSelector, ResolvedSource, ResolvedTarget, Resolver};
use project::handler::{CachePlacement, ExecutionPaths, Locations};
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
    /// adapter store and the out-of-tree project cache isolated
    /// inside it.
    ///
    /// # Panics
    ///
    /// Panics when the tempdir cannot be created.
    #[must_use]
    pub fn bare() -> Self {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let root = tmp.path().canonicalize().expect("canonical tempdir");
        Self::anchored(root, tmp)
    }

    /// A bare directory sharing an existing store root — for tests
    /// that assert a second project reuses installed store entries.
    ///
    /// # Panics
    ///
    /// Panics when the tempdir cannot be created.
    #[must_use]
    pub fn with_store(store: PathBuf) -> Self {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let root = tmp.path().canonicalize().expect("canonical tempdir");
        let locations =
            Locations::explicit(store, CachePlacement::Parent(root.join("project-cache")));
        let paths = ExecutionPaths::new(&root, locations);
        Self {
            root,
            paths,
            _owned: Arc::new(tmp),
        }
    }

    fn anchored(root: PathBuf, tmp: tempfile::TempDir) -> Self {
        let store = root.join("store");
        std::fs::create_dir_all(&store).expect("mkdir store");
        let locations =
            Locations::explicit(store, CachePlacement::Parent(root.join("project-cache")));
        let paths = ExecutionPaths::new(&root, locations);
        Self {
            root,
            paths,
            _owned: Arc::new(tmp),
        }
    }

    /// The provider's isolated global store root.
    #[must_use]
    pub fn store_root(&self) -> &Path {
        self.paths.locations().store_root()
    }

    /// The provider's isolated store entry for `(name, version)`.
    #[must_use]
    pub fn store_entry(&self, name: &str, version: &str) -> PathBuf {
        self.paths.locations().store_entry(name, version)
    }
}

impl project::handler::Anchor for Provider {
    fn paths(&self) -> &ExecutionPaths {
        &self.paths
    }
}

impl Resolver for Provider {
    fn expand(&self, selector: &AdapterSelector, paths: &ExecutionPaths) -> AdapterSelector {
        Resolver::expand(&resolver(), selector, paths)
    }

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

    // The component-deployment ensure kernels: local-component
    // mirroring only — package installation is host-owned and out of
    // reach here, so a package pin ensures without any store write.
    async fn ensure_source(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedSource, Error> {
        project::adapter::ensure::source(stub_metadata, selector, paths, test_now())
    }

    async fn ensure_target(
        &self, selector: &AdapterSelector, paths: &ExecutionPaths,
    ) -> Result<ResolvedTarget, Error> {
        project::adapter::ensure::target(stub_metadata, selector, paths, test_now())
    }
}

/// Deterministic timestamp for ensure provenance stamps.
const fn test_now() -> jiff::Timestamp {
    jiff::Timestamp::UNIX_EPOCH
}

/// The deterministic metadata runner behind [`resolver`], as a plain
/// `fn` for the ensure kernels.
fn stub_metadata(request: &project::adapter::metadata::Request<'_>) -> Result<Metadata, Error> {
    // The target-only fixture exports no source world: dispatching it
    // on the source axis reproduces the dispatch-seam failure a
    // wrong-axis binding hits (no deployed guest answers the id).
    if request.adapter_id.starts_with("source:demo-target") {
        return Err(Error::Diag {
            code: "adapter-metadata-failed",
            detail: format!("no deployed guest exports `{}`", request.adapter_id),
        });
    }
    serde_json::from_str(&metadata_json(request.adapter_id)).map_err(|err| Error::Diag {
        code: "adapter-metadata-failed",
        detail: format!("mock metadata parse {}: {err}", request.adapter_id),
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
/// - `target:demo-target` — a `emery` floor newer than any real
///   binary (the `adapter-cli-too-old` gate);
/// - `target:bad-floor` — an unparseable floor
///   (`adapter-floor-malformed`);
/// - `target:vectis` — declared build inputs plus the full
///   three-platform capability;
/// - anything else — `{}` (no floor, no inputs, no capability).
#[must_use]
pub fn metadata_json(adapter_id: &str) -> String {
    match adapter_id {
        "target:demo-target" => r#"{"emery-floor":"999.0.0"}"#.to_string(),
        "target:bad-floor" => r#"{"emery-floor":"v1"}"#.to_string(),
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

/// Stage a stub adapter component for `name` in the provider's project
/// component cache — the single bare-name probe.
///
/// The stub lands at `<project-cache>/components/<name>.wasm`, so a
/// bare-name resolve against the provider's paths can dispatch the
/// test metadata runner.
///
/// # Panics
///
/// Panics when the cache directory or the stub file cannot be written.
pub fn stage_cached_component(provider: &Provider, name: &str) {
    let components = provider.paths.cache_dir().join("components");
    std::fs::create_dir_all(&components).expect("mkdir component cache");
    std::fs::write(components.join(format!("{name}.wasm")), "{}").expect("write stub component");
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

//! Launcher integration coverage over the public
//! [`emery::launcher::assemble`] seam: anchored mounts created
//! pre-run. Resolver and MCP-route coverage is parked while those
//! launcher exports are unused by the shipped runtime.

use std::path::PathBuf;

use emery::launcher;
use emery_engine::handler::{CachePlacement, ExecutionPaths, Locations};

/// One sandboxed invocation context: a project directory plus explicit
/// store and cache roots, all inside one tempdir.
struct Sandbox {
    root: PathBuf,
    locations: Locations,
    _tmp: tempfile::TempDir,
}

impl Sandbox {
    fn new() -> Self {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let base = tmp.path().canonicalize().expect("canonical tempdir");
        let root = base.join("project");
        let store = base.join("store");
        let cache = base.join("cache");
        for dir in [&root, &store, &cache] {
            std::fs::create_dir_all(dir).expect("mkdir sandbox dir");
        }
        let locations = Locations::explicit(store, CachePlacement::Parent(cache));
        Self {
            root,
            locations,
            _tmp: tmp,
        }
    }

    fn paths(&self) -> ExecutionPaths {
        launcher::assemble(&self.root, self.locations.clone())
    }
}

// ---------------------------------------------------------------------------
// Deployment policy: the mounts anchor from the working directory;
// the writable mount directories are created pre-run.

#[test]
fn mounts_are_well_known() {
    let sandbox = Sandbox::new();
    let paths = sandbox.paths();

    assert_eq!(paths.project_root(), sandbox.root);
    // The writable mount directories are created pre-run so the
    // guest's preopens exist. The global store gets no guest mount —
    // it is host-owned.
    assert!(paths.cache_dir().is_dir());
}

#[test]
fn anchors_at_project_root() {
    let sandbox = Sandbox::new();
    let emery = sandbox.root.join(".emery");
    std::fs::create_dir_all(&emery).expect("mkdir .emery");
    std::fs::write(
        emery.join("project.yaml"),
        format!(
            "name: fixture\nadapter: mock\nemery: {}\nrules: {{}}\n",
            env!("CARGO_PKG_VERSION")
        ),
    )
    .expect("write project.yaml");
    let nested = sandbox.root.join("src/deeply/nested");
    std::fs::create_dir_all(&nested).expect("mkdir nested dir");

    let paths = launcher::assemble(&nested, sandbox.locations.clone());
    assert_eq!(paths.project_root(), sandbox.root);
}

#[test]
fn unanchored_cwd_in_place() {
    // No `project.yaml` ancestor: assembly stays total — it boots
    // in-place at the cwd (pre-init) so `emery init` works and later
    // verbs fail typed in-guest.
    let sandbox = Sandbox::new();
    assert_eq!(sandbox.paths().project_root(), sandbox.root);
}

// // ---------------------------------------------------------------------------
// // Adapter legs: cache seeds and verified store entries resolve
// // offline; everything else is a typed miss — no download path.
//
// #[test]
// fn store_adapter_verify_load() { ... }
// #[test]
// fn mcp_route_maps_routed_ids() { ... }
// (parked: `resolver`, `mcp_route`, and `http_listener` are unused by
//  `src/main.rs` / `examples/runtime.rs`)

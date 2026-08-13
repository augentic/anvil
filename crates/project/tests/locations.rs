//! Carried artifact-layout behavior: explicit construction, the
//! Parent-vs-Project cache placement, root re-anchoring, and the
//! layout formulas over the two well-known roots.

use std::path::{Path, PathBuf};

use project::config::Layout;
use project::handler::{
    CachePlacement, ExecutionPaths, GUEST_CACHE_MOUNT, GUEST_STORE_MOUNT, Locations,
};

fn explicit() -> Locations {
    Locations::explicit(
        PathBuf::from("/layout/store"),
        CachePlacement::Parent(PathBuf::from("/layout/cache")),
    )
}

#[test]
fn store_formulas() {
    let locations = explicit();
    assert_eq!(locations.store_root(), Path::new("/layout/store"));
    assert_eq!(
        locations.store_entry("demo", "1.2.0"),
        PathBuf::from("/layout/store/demo@1.2.0.wasm")
    );
    assert_eq!(
        locations.store_meta("demo", "1.2.0"),
        PathBuf::from("/layout/store/demo@1.2.0.meta")
    );
}

#[test]
fn parent_keys_by_project() {
    let locations = explicit();
    let a = locations.project_cache_dir(Path::new("/some/project/a"));
    let b = locations.project_cache_dir(Path::new("/some/project/b"));
    assert_ne!(a, b, "distinct projects get distinct cache directories");
    assert_eq!(a.parent(), Some(Path::new("/layout/cache")));
    assert_eq!(a, locations.project_cache_dir(Path::new("/some/project/a")), "stable per project");
    assert_eq!(
        locations.component(Path::new("/some/project/a"), "demo"),
        a.join("components/demo.wasm"),
    );
}

#[test]
fn project_placement() {
    let locations = Locations::explicit(
        PathBuf::from("/store"),
        CachePlacement::Project(PathBuf::from("/cache")),
    );
    assert_eq!(locations.project_cache_dir(Path::new("/some/project/a")), PathBuf::from("/cache"));
    assert_eq!(
        locations.project_cache_dir(Path::new("/some/project/b")),
        PathBuf::from("/cache"),
        "one project per deployment: no project-id keying"
    );
}

#[test]
fn root_preserves_placement() {
    let host = ExecutionPaths::new("/some/project/a", explicit());
    let moved = host.with_root("/some/project/b");
    assert_eq!(moved.project_root(), Path::new("/some/project/b"));
    assert_ne!(moved.cache_dir(), host.cache_dir(), "a host parent re-keys per project");

    let guest = ExecutionPaths::guest();
    let moved = guest.with_root("/workspace/slot");
    assert_eq!(
        moved.cache_dir(),
        PathBuf::from(GUEST_CACHE_MOUNT),
        "a guest per-project cache stays the one mounted preopen"
    );
}

#[test]
fn guest_paths_bind_preopens() {
    let paths = ExecutionPaths::guest();
    assert_eq!(paths.project_root(), Path::new("."));
    assert_eq!(paths.change_root(), Layout::new(Path::new(".")).change_root());
    assert!(!paths.is_detached());
    assert_eq!(paths.locations().store_root(), Path::new(GUEST_STORE_MOUNT));
    assert_eq!(paths.cache_dir(), PathBuf::from(GUEST_CACHE_MOUNT));
}

#[test]
fn detached_constructor() {
    let paths = ExecutionPaths::detached("/tmp/change-home", explicit());
    assert_eq!(paths.project_root(), Path::new("/tmp/change-home"));
    assert_eq!(paths.change_root(), Path::new("/tmp/change-home"));
    assert!(paths.is_detached());
}

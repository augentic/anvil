//! Integration coverage for the operator-authored `parts.yaml` input
//! (`workflow_lib::design_system::Parts`). Re-homed from the former
//! `design_system/tests.rs` unit module. The private `from_yaml` parser is
//! exercised through the public `Parts::load` (write-to-disk then load), which
//! is the only path operators reach.

use workflow_lib::design_system::Parts;

const TAB_BAR_PART: &str = "version: 1
parts:
  tab-bar:
    description: Bottom navigation across primary sections.
    group:
      active-when: \"$route\"
      items:
        - icon-button: { bind: home, event: Navigate(Home) }
        - icon-button: { bind: search, event: Navigate(Search) }
";

/// Write `body` to the canonical `parts.yaml` path under a fresh project root
/// and return the temp dir handle (kept alive by the caller).
fn staged(body: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Parts::path_in(dir.path());
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, body).expect("write parts.yaml");
    dir
}

#[test]
fn load_returns_none_when_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(Parts::load(dir.path()).expect("no error").is_none());
}

#[test]
fn load_parses_part_from_disk() {
    let dir = staged(TAB_BAR_PART);
    let parts = Parts::load(dir.path()).expect("load").expect("present");
    assert_eq!(parts.version, 1);
    assert_eq!(parts.parts.len(), 1);
    assert!(parts.parts.contains_key("tab-bar"));
    assert_eq!(parts.description_of("tab-bar"), Some("Bottom navigation across primary sections."));
    assert!(parts.parts.get("tab-bar").expect("part").group.get("items").is_some());
}

#[test]
fn load_rejects_missing_group() {
    let dir = staged("version: 1\nparts:\n  tab-bar:\n    description: no group\n");
    Parts::load(dir.path()).expect_err("a part without a `group` must fail schema validation");
}

#[test]
fn load_rejects_non_kebab_slug() {
    let dir =
        staged("version: 1\nparts:\n  TabBar:\n    group:\n      items:\n        - text: {}\n");
    Parts::load(dir.path()).expect_err("a non-kebab part slug must fail schema validation");
}

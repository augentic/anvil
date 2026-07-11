//! Integration coverage for the design-system YAML inputs: the
//! operator-authored `parts.yaml` (`workflow::design_system::Parts`)
//! and the agent-written component catalog
//! (`workflow::design_system::ComponentsCatalog`). Both are exercised
//! through the public write-to-disk-then-`load` path, which is the only
//! path operators reach.

use std::collections::BTreeMap;

use workflow::design_system::{ComponentEntry, ComponentStatus, ComponentsCatalog, Parts};

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

// ---------------------------------------------------------------------------
// ComponentsCatalog
// ---------------------------------------------------------------------------

/// Write `body` to the canonical `components.yaml` path under a fresh
/// project root and return the temp dir handle.
fn staged_catalog(body: &str) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = ComponentsCatalog::path_in(dir.path());
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, body).expect("write components.yaml");
    dir
}

fn confirmed(description: Option<&str>) -> ComponentEntry {
    ComponentEntry {
        status: ComponentStatus::Confirmed,
        description: description.map(str::to_string),
        fingerprint: None,
    }
}

fn rejected() -> ComponentEntry {
    ComponentEntry {
        status: ComponentStatus::Rejected,
        description: None,
        fingerprint: None,
    }
}

#[test]
fn catalog_parse_and_validate() {
    // `load` returns `None` when the file is absent.
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(ComponentsCatalog::load(dir.path()).expect("no error").is_none());

    // A well-formed catalog parses; `status_of` resolves known/unknown slugs.
    let dir = staged_catalog(
        "version: 1\ncomponents:\n  tab-bar:\n    status: confirmed\n    description: \"Bottom \
         navigation\"\n  hero-banner:\n    status: rejected\n",
    );
    let catalog = ComponentsCatalog::load(dir.path()).expect("load").expect("present");
    assert_eq!(catalog.version, 1);
    assert_eq!(catalog.components.len(), 2);
    assert_eq!(catalog.status_of("tab-bar"), Some(ComponentStatus::Confirmed));
    assert_eq!(catalog.status_of("hero-banner"), Some(ComponentStatus::Rejected));
    assert_eq!(catalog.status_of("missing"), None);

    // An empty components map is valid.
    let dir = staged_catalog("version: 1\ncomponents: {}\n");
    let empty = ComponentsCatalog::load(dir.path()).expect("load").expect("present");
    assert!(empty.components.is_empty());

    // A 64-hex fingerprint field round-trips.
    let fp = "a".repeat(64);
    let dir = staged_catalog(&format!(
        "version: 1\ncomponents:\n  tab-bar:\n    status: confirmed\n    fingerprint: \"{fp}\"\n"
    ));
    let with_fp = ComponentsCatalog::load(dir.path()).expect("load").expect("present");
    assert_eq!(with_fp.components.get("tab-bar").and_then(|e| e.fingerprint.clone()), Some(fp));

    // Schema rejections: missing version, invalid status, non-kebab slug,
    // malformed fingerprint.
    let dir = staged_catalog("components:\n  tab-bar:\n    status: confirmed\n");
    ComponentsCatalog::load(dir.path()).expect_err("missing version");
    let dir = staged_catalog("version: 1\ncomponents:\n  tab-bar:\n    status: pending\n");
    ComponentsCatalog::load(dir.path()).expect_err("invalid status");
    let dir = staged_catalog("version: 1\ncomponents:\n  TabBar:\n    status: confirmed\n");
    ComponentsCatalog::load(dir.path()).expect_err("non-kebab slug");
    let dir = staged_catalog(
        "version: 1\ncomponents:\n  tab-bar:\n    status: confirmed\n    fingerprint: \
         \"not-a-hash\"\n",
    );
    ComponentsCatalog::load(dir.path()).expect_err("malformed fingerprint");
}

#[test]
fn catalog_upsert_and_persist() {
    // `upsert_bound` adds a confirmed slug carrying its description + fingerprint.
    let fp = "f".repeat(64);
    let mut catalog = ComponentsCatalog::empty();
    catalog.upsert_bound("tab-bar", &fp, Some("Bottom nav".to_string()));
    assert_eq!(catalog.status_of("tab-bar"), Some(ComponentStatus::Confirmed));
    let entry = catalog.components.get("tab-bar").expect("entry");
    assert_eq!(entry.description.as_deref(), Some("Bottom nav"));
    assert_eq!(entry.fingerprint.as_deref(), Some(fp.as_str()));

    // It never reconfirms a rejected slug, and leaves its fields untouched.
    let mut catalog = ComponentsCatalog {
        version: 1,
        components: BTreeMap::from([("tab-bar".to_string(), rejected())]),
    };
    catalog.upsert_bound("tab-bar", &"f".repeat(64), Some("ignored".to_string()));
    assert_eq!(catalog.status_of("tab-bar"), Some(ComponentStatus::Rejected));
    let entry = catalog.components.get("tab-bar").expect("entry");
    assert!(entry.description.is_none() && entry.fingerprint.is_none());

    // It keeps an existing confirmed entry's description and does not re-fingerprint.
    let mut catalog = ComponentsCatalog {
        version: 1,
        components: BTreeMap::from([("tab-bar".to_string(), confirmed(Some("original")))]),
    };
    catalog.upsert_bound("tab-bar", &"f".repeat(64), Some("replacement".to_string()));
    let entry = catalog.components.get("tab-bar").expect("entry");
    assert_eq!(entry.description.as_deref(), Some("original"));
    assert!(entry.fingerprint.is_none());

    // `fingerprint_index` maps stored fingerprints; a fingerprint-less entry
    // contributes nothing.
    let fp = "c".repeat(64);
    let mut catalog = ComponentsCatalog::empty();
    catalog.upsert_bound("tab-bar", &fp, None);
    catalog.components.insert("hero".to_string(), confirmed(None));
    let index = catalog.fingerprint_index();
    assert_eq!(index.get(fp.as_str()), Some(&"tab-bar"));
    assert_eq!(index.len(), 1);

    // `save` then `load` round-trips through disk.
    let dir = tempfile::tempdir().expect("tempdir");
    let mut catalog = ComponentsCatalog::empty();
    catalog.upsert_bound("tab-bar", &"d".repeat(64), Some("Bottom nav".to_string()));
    catalog.save(dir.path()).expect("save");
    let reloaded = ComponentsCatalog::load(dir.path()).expect("load").expect("present");
    assert_eq!(reloaded, catalog);
    assert_eq!(reloaded.status_of("tab-bar"), Some(ComponentStatus::Confirmed));
}

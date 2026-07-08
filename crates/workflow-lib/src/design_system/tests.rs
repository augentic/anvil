use super::*;

// The operator-authored `parts.yaml` input is exercised through the public
// `Parts::load` in `crates/workflow-lib/tests/design_system_parts.rs`. What stays
// here is the agent-written `ComponentsCatalog`, collapsed into two
// table-style tests — its `from_yaml` parser is private, so it cannot re-home.
// Every former single-purpose assertion is preserved.

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

    let path = Path::new("test.yaml");

    // A well-formed catalog parses; `status_of` resolves known/unknown slugs.
    let yaml = "version: 1\ncomponents:\n  tab-bar:\n    status: confirmed\n    description: \"Bottom navigation\"\n  hero-banner:\n    status: rejected\n";
    let catalog = ComponentsCatalog::from_yaml(yaml, path).expect("valid");
    assert_eq!(catalog.version, 1);
    assert_eq!(catalog.components.len(), 2);
    assert_eq!(catalog.status_of("tab-bar"), Some(ComponentStatus::Confirmed));
    assert_eq!(catalog.status_of("hero-banner"), Some(ComponentStatus::Rejected));
    assert_eq!(catalog.status_of("missing"), None);

    // An empty components map is valid.
    let empty = ComponentsCatalog::from_yaml("version: 1\ncomponents: {}\n", path).expect("valid");
    assert!(empty.components.is_empty());

    // A 64-hex fingerprint field round-trips.
    let fp = "a".repeat(64);
    let with_fp = ComponentsCatalog::from_yaml(
        &format!(
            "version: 1\ncomponents:\n  tab-bar:\n    status: confirmed\n    fingerprint: \"{fp}\"\n"
        ),
        path,
    )
    .expect("valid");
    assert_eq!(with_fp.components.get("tab-bar").and_then(|e| e.fingerprint.clone()), Some(fp));

    // Schema rejections: missing version, invalid status, non-kebab slug,
    // malformed fingerprint.
    ComponentsCatalog::from_yaml("components:\n  tab-bar:\n    status: confirmed\n", path)
        .expect_err("missing version");
    ComponentsCatalog::from_yaml(
        "version: 1\ncomponents:\n  tab-bar:\n    status: pending\n",
        path,
    )
    .expect_err("invalid status");
    ComponentsCatalog::from_yaml(
        "version: 1\ncomponents:\n  TabBar:\n    status: confirmed\n",
        path,
    )
    .expect_err("non-kebab slug");
    ComponentsCatalog::from_yaml(
        "version: 1\ncomponents:\n  tab-bar:\n    status: confirmed\n    fingerprint: \"not-a-hash\"\n",
        path,
    )
    .expect_err("malformed fingerprint");
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

use super::*;

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
fn load_returns_none_when_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = ComponentsCatalog::load(dir.path()).expect("no error");
    assert!(result.is_none());
}

#[test]
fn status_of_returns_correct_variant() {
    let catalog = ComponentsCatalog {
        version: 1,
        components: BTreeMap::from([("tab-bar".to_string(), confirmed(Some("Bottom nav")))]),
    };
    assert_eq!(catalog.status_of("tab-bar"), Some(ComponentStatus::Confirmed));
    assert_eq!(catalog.status_of("missing"), None);
}

#[test]
fn round_trip_yaml() {
    let yaml = "version: 1\ncomponents:\n  tab-bar:\n    status: confirmed\n    description: \"Bottom navigation\"\n  hero-banner:\n    status: rejected\n";
    let path = Path::new("test.yaml");
    let catalog = ComponentsCatalog::from_yaml(yaml, path).expect("valid");
    assert_eq!(catalog.version, 1);
    assert_eq!(catalog.components.len(), 2);
    assert_eq!(catalog.status_of("tab-bar"), Some(ComponentStatus::Confirmed));
    assert_eq!(catalog.status_of("hero-banner"), Some(ComponentStatus::Rejected));
}

#[test]
fn rejects_missing_version() {
    let yaml = "components:\n  tab-bar:\n    status: confirmed\n";
    let path = Path::new("test.yaml");
    ComponentsCatalog::from_yaml(yaml, path).unwrap_err();
}

#[test]
fn rejects_invalid_status() {
    let yaml = "version: 1\ncomponents:\n  tab-bar:\n    status: pending\n";
    let path = Path::new("test.yaml");
    ComponentsCatalog::from_yaml(yaml, path).unwrap_err();
}

#[test]
fn rejects_non_kebab_slug() {
    let yaml = "version: 1\ncomponents:\n  TabBar:\n    status: confirmed\n";
    let path = Path::new("test.yaml");
    ComponentsCatalog::from_yaml(yaml, path).unwrap_err();
}

#[test]
fn empty_components_is_valid() {
    let yaml = "version: 1\ncomponents: {}\n";
    let path = Path::new("test.yaml");
    let catalog = ComponentsCatalog::from_yaml(yaml, path).expect("valid");
    assert!(catalog.components.is_empty());
}

#[test]
fn accepts_fingerprint_field() {
    let fp = "a".repeat(64);
    let yaml = format!(
        "version: 1\ncomponents:\n  tab-bar:\n    status: confirmed\n    fingerprint: \"{fp}\"\n"
    );
    let path = Path::new("test.yaml");
    let catalog = ComponentsCatalog::from_yaml(&yaml, path).expect("valid");
    assert_eq!(catalog.components.get("tab-bar").and_then(|e| e.fingerprint.clone()), Some(fp));
}

#[test]
fn rejects_malformed_fingerprint() {
    let yaml = "version: 1\ncomponents:\n  tab-bar:\n    status: confirmed\n    fingerprint: \"not-a-hash\"\n";
    let path = Path::new("test.yaml");
    ComponentsCatalog::from_yaml(yaml, path).unwrap_err();
}

#[test]
fn upsert_bound_adds_slug_with_fp() {
    let fp = "f".repeat(64);
    let mut catalog = ComponentsCatalog::empty();
    catalog.upsert_bound("tab-bar", &fp, Some("Bottom nav".to_string()));
    assert_eq!(catalog.status_of("tab-bar"), Some(ComponentStatus::Confirmed));
    let entry = catalog.components.get("tab-bar").expect("entry");
    assert_eq!(entry.description.as_deref(), Some("Bottom nav"));
    assert_eq!(entry.fingerprint.as_deref(), Some(fp.as_str()));
}

#[test]
fn upsert_bound_never_reconfirms_rejected() {
    let mut catalog = ComponentsCatalog {
        version: 1,
        components: BTreeMap::from([("tab-bar".to_string(), rejected())]),
    };
    catalog.upsert_bound("tab-bar", &"f".repeat(64), Some("ignored".to_string()));
    assert_eq!(catalog.status_of("tab-bar"), Some(ComponentStatus::Rejected));
    let entry = catalog.components.get("tab-bar").expect("entry");
    assert!(entry.description.is_none());
    assert!(entry.fingerprint.is_none());
}

#[test]
fn upsert_bound_keeps_confirmed() {
    let mut catalog = ComponentsCatalog {
        version: 1,
        components: BTreeMap::from([("tab-bar".to_string(), confirmed(Some("original")))]),
    };
    catalog.upsert_bound("tab-bar", &"f".repeat(64), Some("replacement".to_string()));
    let entry = catalog.components.get("tab-bar").expect("entry");
    assert_eq!(entry.description.as_deref(), Some("original"));
    assert!(entry.fingerprint.is_none());
}

#[test]
fn fingerprint_index_maps_stored() {
    let fp = "c".repeat(64);
    let mut catalog = ComponentsCatalog::empty();
    catalog.upsert_bound("tab-bar", &fp, None);
    // A hand-authored entry without a fingerprint contributes nothing.
    catalog.components.insert("hero".to_string(), confirmed(None));
    let index = catalog.fingerprint_index();
    assert_eq!(index.get(fp.as_str()), Some(&"tab-bar"));
    assert_eq!(index.len(), 1);
}

#[test]
fn save_round_trips_through_load() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut catalog = ComponentsCatalog::empty();
    catalog.upsert_bound("tab-bar", &"d".repeat(64), Some("Bottom nav".to_string()));
    catalog.save(dir.path()).expect("save");

    let reloaded = ComponentsCatalog::load(dir.path()).expect("load").expect("present");
    assert_eq!(reloaded, catalog);
    assert_eq!(reloaded.status_of("tab-bar"), Some(ComponentStatus::Confirmed));
}

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

#[test]
fn parts_load_returns_none_when_absent() {
    let dir = tempfile::tempdir().expect("tempdir");
    assert!(Parts::load(dir.path()).expect("no error").is_none());
}

#[test]
fn parts_from_yaml_parses_part() {
    let parts = Parts::from_yaml(TAB_BAR_PART, Path::new("parts.yaml")).expect("valid");
    assert_eq!(parts.version, 1);
    assert_eq!(parts.parts.len(), 1);
    assert_eq!(parts.description_of("tab-bar"), Some("Bottom navigation across primary sections."));
    assert!(parts.parts.get("tab-bar").expect("part").group.get("items").is_some());
}

#[test]
fn parts_from_yaml_rejects_missing_group() {
    let yaml = "version: 1\nparts:\n  tab-bar:\n    description: no group\n";
    Parts::from_yaml(yaml, Path::new("parts.yaml")).unwrap_err();
}

#[test]
fn parts_from_yaml_rejects_non_kebab_slug() {
    let yaml = "version: 1\nparts:\n  TabBar:\n    group:\n      items:\n        - text: {}\n";
    Parts::from_yaml(yaml, Path::new("parts.yaml")).unwrap_err();
}

#[test]
fn parts_load_round_trips_from_disk() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = Parts::path_in(dir.path());
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, TAB_BAR_PART).expect("write parts.yaml");
    let parts = Parts::load(dir.path()).expect("load").expect("present");
    assert!(parts.parts.contains_key("tab-bar"));
}

use super::*;
use crate::Platform;

#[test]
fn round_trips_yaml_empties_elided() {
    let lock = TopologyLock::from_projects(vec![
        TopologyProject {
            name: "identity-contracts".to_string(),
            target: "contracts@1.0.0".to_string(),
            description: Some("Contracts crate.".to_string()),
            surface: vec![Surface {
                domain: "identity-api".to_string(),
                requirements: vec!["Authenticate user".to_string()],
                more: None,
            }],
            recent: Vec::new(),
            decisions: Vec::new(),
            decisions_more: None,
            platforms: Vec::new(),
        },
        TopologyProject {
            name: "identity-service".to_string(),
            target: "omnia@1.0.0".to_string(),
            description: None,
            surface: Vec::new(),
            recent: Vec::new(),
            decisions: Vec::new(),
            decisions_more: None,
            platforms: Vec::new(),
        },
    ]);

    let yaml = serde_saphyr::to_string(&lock).expect("serialize lock");
    assert!(yaml.contains("name: identity-contracts"), "{yaml}");
    assert!(!yaml.contains("recent:"), "empty recent elided: {yaml}");
    assert!(!yaml.contains("platforms:"), "empty platforms elided: {yaml}");

    let parsed: TopologyLock = serde_saphyr::from_str(&yaml).expect("round-trip");
    assert_eq!(parsed, lock);
}

#[test]
fn round_trips_with_platforms() {
    let lock = TopologyLock::from_projects(vec![TopologyProject {
        name: "mobile-app".to_string(),
        target: "vectis@1.0.0".to_string(),
        description: Some("Cross-platform mobile app.".to_string()),
        surface: Vec::new(),
        recent: Vec::new(),
        decisions: Vec::new(),
        decisions_more: None,
        platforms: vec![Platform::Core, Platform::Ios, Platform::Android],
    }]);

    let yaml = serde_saphyr::to_string(&lock).expect("serialize lock");
    assert!(yaml.contains("platforms:"), "platforms present: {yaml}");
    assert!(yaml.contains("core"), "{yaml}");
    assert!(yaml.contains("ios"), "{yaml}");
    assert!(yaml.contains("android"), "{yaml}");

    let parsed: TopologyLock = serde_saphyr::from_str(&yaml).expect("round-trip");
    assert_eq!(parsed, lock);
}

#[test]
fn save_then_load_is_identity() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("topology.lock");
    let lock = TopologyLock::from_projects(vec![TopologyProject {
        name: "svc".to_string(),
        target: "omnia@1.0.0".to_string(),
        description: None,
        surface: Vec::new(),
        recent: Vec::new(),
        decisions: Vec::new(),
        decisions_more: None,
        platforms: Vec::new(),
    }]);

    lock.save(&path).expect("save");
    let loaded = TopologyLock::load(&path).expect("load").expect("present");
    assert_eq!(loaded, lock);
}

#[test]
fn save_then_load_with_platforms() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("topology.lock");
    let lock = TopologyLock::from_projects(vec![TopologyProject {
        name: "mobile-app".to_string(),
        target: "vectis@1.0.0".to_string(),
        description: None,
        surface: Vec::new(),
        recent: Vec::new(),
        decisions: Vec::new(),
        decisions_more: None,
        platforms: vec![Platform::Core, Platform::Ios, Platform::Android],
    }]);

    lock.save(&path).expect("save");
    let loaded = TopologyLock::load(&path).expect("load").expect("present");
    assert_eq!(loaded, lock);
}

#[test]
fn missing_file_is_none() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("absent.lock");
    assert_eq!(TopologyLock::load(&path).expect("load"), None);
}

#[test]
fn version_too_new_errors() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("topology.lock");
    fs::write(&path, "version: 99\nprojects: []\n").expect("write");
    let err = TopologyLock::load(&path).expect_err("too new");
    assert!(format!("{err:?}").contains("topology-lock-version-too-new"), "{err:?}");
}

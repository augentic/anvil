use std::collections::BTreeMap;
use std::fs;

use tempfile::tempdir;

use super::*;
use crate::config::Layout;

#[test]
fn with_state_propagates_error_skips_write() {
    let tmp = tempdir().expect("tempdir");
    let layout = Layout::new(tmp.path());
    let initial = Registry {
        version: 1,
        projects: Vec::new(),
    };
    yaml_write(&layout.registry_path(), &initial).expect("seed registry.yaml");
    let before = fs::read_to_string(layout.registry_path()).expect("read seed");

    let err = with_state::<Registry, (), _>(layout, "registry.yaml", |_| {
        Err(Error::Diag {
            code: "test-abort",
            detail: "abort".into(),
        })
    })
    .expect_err("closure error must propagate");
    assert!(matches!(
        err,
        Error::Diag {
            code: "test-abort",
            ..
        }
    ));
    let after = fs::read_to_string(layout.registry_path()).expect("read after");
    assert_eq!(before, after, "registry.yaml must be byte-identical when the closure errs");
}

#[test]
fn with_state_missing_errors() {
    let tmp = tempdir().expect("tempdir");
    let layout = Layout::new(tmp.path());
    let err = with_state::<Registry, (), _>(layout, "registry.yaml", |_| Ok(()))
        .expect_err("absent file must error");
    match err {
        Error::ArtifactNotFound { kind, path } => {
            assert_eq!(kind, "registry.yaml");
            assert_eq!(path, layout.registry_path());
        }
        other => panic!("expected ArtifactNotFound, got {other:?}"),
    }
}

#[test]
fn with_state_require_existing_round_trips() {
    let tmp = tempdir().expect("tempdir");
    let layout = Layout::new(tmp.path());
    let initial = Registry {
        version: 1,
        projects: Vec::new(),
    };
    yaml_write(&layout.registry_path(), &initial).expect("seed registry.yaml");

    with_state::<Registry, (), _>(layout, "registry.yaml", |reg| {
        reg.projects.push(crate::registry::RegistryProject {
            name: "alpha".into(),
            url: ".".into(),
            adapter: Some("omnia@1.0.0".into()),
            description: None,
            contracts: None,
            greenfield_seed: None,
        });
        Ok(())
    })
    .expect("mutate ok");

    let reloaded = Registry::load(tmp.path()).expect("load").expect("present");
    assert_eq!(reloaded.projects.len(), 1);
    assert_eq!(reloaded.projects[0].name, "alpha");
}

#[test]
fn load_maps_not_initialized_to_none() {
    let tmp = tempdir().expect("tempdir");
    let layout = Layout::new(tmp.path());
    let loaded = <ProjectConfig as AtomicYaml>::load_state(layout).expect("load ok");
    assert!(loaded.is_none(), "absent project.yaml must surface as None");
}

#[test]
fn load_round_trips_when_present() {
    let tmp = tempdir().expect("tempdir");
    let layout = Layout::new(tmp.path());
    let cfg = ProjectConfig {
        name: "demo".into(),
        description: None,
        adapter: Some("omnia".into()),
        specify_version: None,
        rules: BTreeMap::new(),
        tools: Vec::new(),
        platforms: Vec::new(),
        workspace: false,
    };
    fs::create_dir_all(layout.specify_dir()).expect("create .specify");
    yaml_write(&layout.config_path(), &cfg).expect("seed project.yaml");
    let loaded =
        <ProjectConfig as AtomicYaml>::load_state(layout).expect("load ok").expect("present");
    assert_eq!(loaded.name, "demo");
}

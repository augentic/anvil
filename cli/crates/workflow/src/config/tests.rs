use std::fs;

use proptest::prelude::*;
use tempfile::tempdir;

use super::*;

fn write_config(dir: &Path, yaml: &str) {
    let specify = dir.join(".specify");
    fs::create_dir_all(&specify).expect("create .specify");
    fs::write(specify.join("project.yaml"), yaml).expect("write project.yaml");
}

#[test]
fn specify_subpaths() {
    let base = Path::new("/a/b");
    let layout = Layout::new(base);
    assert_eq!(layout.project_dir(), base);
    assert_eq!(layout.specify_dir(), PathBuf::from("/a/b/.specify"));
    assert_eq!(layout.config_path(), PathBuf::from("/a/b/.specify/project.yaml"));
    assert_eq!(layout.slices_dir(), PathBuf::from("/a/b/.specify/slices"));
    assert_eq!(layout.topology_lock_path(), PathBuf::from("/a/b/.specify/topology.lock"));
    assert_eq!(layout.registry_path(), PathBuf::from("/a/b/registry.yaml"));
    assert_eq!(layout.plan_path(), PathBuf::from("/a/b/plan.yaml"));
    assert_eq!(layout.change_brief_path(), PathBuf::from("/a/b/change.md"));
    assert_eq!(layout.discovery_path(), PathBuf::from("/a/b/discovery.md"));
    // The cache is regenerable, machine-owned state that lives out-of-tree;
    // `cache_dir` delegates to the per-project OS-cache resolver.
    assert_eq!(layout.cache_dir(), specify_schema::cache::project_cache_dir(base));
    assert_eq!(layout.archive_dir(), PathBuf::from("/a/b/.specify/archive"));
}

#[test]
fn plan_dir_moves_plan_artifacts_only() {
    // Workspace routing: phase verbs run inside a slot while the plan
    // artifacts live at the initiating workspace. The override moves
    // exactly the three plan-root artifacts; every `.specify/` path
    // stays anchored to the project (slot) root.
    let slot = Path::new("/ws/workspace/orders");
    let workspace = Path::new("/ws");
    let layout = Layout::new(slot).with_plan_dir(Some(workspace));
    assert_eq!(layout.project_dir(), slot);
    assert_eq!(layout.plan_dir(), workspace);
    assert_eq!(layout.plan_path(), PathBuf::from("/ws/plan.yaml"));
    assert_eq!(layout.change_brief_path(), PathBuf::from("/ws/change.md"));
    assert_eq!(layout.discovery_path(), PathBuf::from("/ws/discovery.md"));
    assert_eq!(layout.specify_dir(), PathBuf::from("/ws/workspace/orders/.specify"));
    assert_eq!(layout.slices_dir(), PathBuf::from("/ws/workspace/orders/.specify/slices"));
    assert_eq!(layout.registry_path(), PathBuf::from("/ws/workspace/orders/registry.yaml"));
}

#[test]
fn plan_dir_none_keeps_project_root() {
    let base = Path::new("/a/b");
    let layout = Layout::new(base).with_plan_dir(None);
    assert_eq!(layout.plan_dir(), base);
    assert_eq!(layout.plan_path(), PathBuf::from("/a/b/plan.yaml"));
}

fn sample_cfg(rules: BTreeMap<String, String>) -> ProjectConfig {
    ProjectConfig {
        name: "demo".to_string(),
        description: None,
        adapter: Some("omnia".to_string()),
        specify_version: None,
        rules,
        tools: Vec::new(),
        platforms: Vec::new(),
        workspace: false,
    }
}

#[test]
fn load_not_initialized_when_missing() {
    let tmp = tempdir().unwrap();
    let err = ProjectConfig::load(tmp.path()).expect_err("missing file errs");
    assert!(matches!(err, Error::NotInitialized));
}

#[test]
fn load_refuses_future_specify_version() {
    let tmp = tempdir().unwrap();
    write_config(tmp.path(), "name: demo\nadapter: omnia\nspecify_version: \"99.0.0\"\n");
    let err = ProjectConfig::load(tmp.path()).expect_err("future version rejected");
    match err {
        Error::CliTooOld { required, found } => {
            assert_eq!(required, "99.0.0");
            assert_eq!(found, env!("CARGO_PKG_VERSION"));
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn load_accepts_floor_lte_current() {
    let tmp = tempdir().unwrap();
    write_config(tmp.path(), "name: demo\nadapter: omnia\nspecify_version: \"0.0.1\"\n");
    ProjectConfig::load(tmp.path()).expect("older version loads");

    let tmp = tempdir().unwrap();
    let exact = env!("CARGO_PKG_VERSION");
    write_config(
        tmp.path(),
        &format!("name: demo\nadapter: omnia\nspecify_version: \"{exact}\"\n"),
    );
    ProjectConfig::load(tmp.path()).expect("exact version loads");
}

#[test]
fn load_same_major_injected_current() {
    let tmp = tempdir().unwrap();
    write_config(tmp.path(), "name: demo\nadapter: omnia\nspecify_version: \"2.0.0\"\n");
    let cfg = ProjectConfig::load_with_current(tmp.path(), "2.4.1").expect("same major loads");
    assert_eq!(cfg.specify_version.as_deref(), Some("2.0.0"));
}

#[test]
fn load_allows_invalid_pinned_version() {
    let tmp = tempdir().unwrap();
    write_config(tmp.path(), "name: demo\nadapter: omnia\nspecify_version: not-a-semver\n");
    let cfg = ProjectConfig::load(tmp.path()).expect("unparseable version is permissive");
    assert_eq!(cfg.specify_version.as_deref(), Some("not-a-semver"));
}

#[test]
fn hub_field_defaults_false_round_trips() {
    let tmp = tempdir().unwrap();
    write_config(tmp.path(), "name: demo\nadapter: omnia\n");
    let cfg = ProjectConfig::load(tmp.path()).expect("loads");
    assert!(!cfg.workspace, "workspace must default to false when absent");
    assert_eq!(cfg.adapter.as_deref(), Some("omnia"));
    assert!(cfg.tools.is_empty(), "tools must default empty when absent");

    let tmp = tempdir().unwrap();
    write_config(tmp.path(), "name: demo\nworkspace: true\n");
    let cfg = ProjectConfig::load(tmp.path()).expect("loads");
    assert!(cfg.workspace, "workspace: true must round-trip through deserialize");
    assert!(cfg.adapter.is_none(), "hub project.yaml must omit adapter:");
}

#[test]
fn workspace_omitted_when_false() {
    let cfg = ProjectConfig {
        name: "demo".to_string(),
        description: None,
        adapter: Some("omnia".to_string()),
        specify_version: None,
        rules: BTreeMap::new(),
        tools: Vec::new(),
        platforms: Vec::new(),
        workspace: false,
    };
    let yaml = serde_saphyr::to_string(&cfg).expect("serialise");
    assert!(!yaml.contains("workspace:"), "workspace: false should be omitted, got:\n{yaml}");
    assert!(yaml.contains("adapter: omnia"), "adapter: must serialise, got:\n{yaml}");
}

#[test]
fn hub_field_serialised_when_true() {
    let cfg = ProjectConfig {
        name: "platform".to_string(),
        description: None,
        adapter: None,
        specify_version: None,
        rules: BTreeMap::new(),
        tools: Vec::new(),
        platforms: Vec::new(),
        workspace: true,
    };
    let yaml = serde_saphyr::to_string(&cfg).expect("serialise");
    assert!(yaml.contains("workspace: true"), "workspace: true must serialise, got:\n{yaml}");
    assert!(!yaml.contains("adapter:"), "hub project.yaml must omit `adapter:`, got:\n{yaml}");
}

#[test]
fn tools_field_round_trips() {
    let tmp = tempdir().unwrap();
    write_config(
        tmp.path(),
        "name: demo\nadapter: omnia\ntools:\n  - name: contract\n    version: 1.0.0\n    source: https://example.com/contract.wasm\n",
    );
    let cfg = ProjectConfig::load(tmp.path()).expect("loads");
    assert_eq!(cfg.tools.len(), 1);
    assert_eq!(cfg.tools[0].name, "contract");
    assert!(matches!(
        &cfg.tools[0].source,
        specify_extension_manifest::ExtensionSource::HttpsUri(uri) if uri == "https://example.com/contract.wasm"
    ));

    let yaml = serde_saphyr::to_string(&cfg).expect("serialise");
    assert!(yaml.contains("tools:"), "tools should serialise when present, got:\n{yaml}");
    assert!(
        yaml.contains("source: https://example.com/contract.wasm"),
        "tool source should stay in string form, got:\n{yaml}"
    );
}

#[test]
fn tools_field_omitted_when_empty() {
    let cfg = sample_cfg(BTreeMap::new());
    let yaml = serde_saphyr::to_string(&cfg).expect("serialise");
    assert!(!yaml.contains("tools:"), "empty tools should be omitted, got:\n{yaml}");
}

fn platform_with_slot(peer: &str) -> tempfile::TempDir {
    let tmp = tempdir().unwrap();
    fs::create_dir_all(tmp.path().join(".specify")).unwrap();
    fs::write(tmp.path().join(".specify").join("project.yaml"), "workspace: true\n").unwrap();
    fs::create_dir_all(tmp.path().join("workspace").join(peer)).unwrap();
    tmp
}

#[test]
fn slot_detects_slot_root() {
    let tmp = platform_with_slot("orders");
    assert!(is_slot(&tmp.path().join("workspace").join("orders")));
}

#[test]
fn workspace_clone_detects_nested() {
    let tmp = platform_with_slot("orders");
    let nested = tmp.path().join("workspace").join("orders").join("src").join("service");
    fs::create_dir_all(&nested).unwrap();
    assert!(is_slot(&nested));
}

#[test]
fn slot_rejects_non_slot_paths() {
    let tmp = platform_with_slot("orders");
    assert!(!is_slot(tmp.path()));
    assert!(!is_slot(&tmp.path().join(".specify")));
    assert!(!is_slot(&tmp.path().join("workspace")));
}

#[test]
fn slot_rejects_workspace_dir_no_config() {
    let tmp = tempdir().unwrap();
    let project = tmp.path().join("workspace").join("orders");
    fs::create_dir_all(&project).unwrap();
    assert!(!is_slot(&project), "no platform .specify/project.yaml at the grandparent");
}

#[test]
fn find_root_walks_up_to_specify_project() {
    let tmp = tempdir().unwrap();
    let root = tmp.path();
    let nested = root.join("sub").join("dir");
    fs::create_dir_all(&nested).expect("mkdir nested");
    write_config(root, "name: demo\nadapter: omnia\n");

    assert_eq!(ProjectConfig::find_root(root).as_deref(), Some(root));
    assert_eq!(ProjectConfig::find_root(&nested).as_deref(), Some(root));
}

#[test]
fn find_root_none_outside_tree() {
    let tmp = tempdir().unwrap();
    assert!(ProjectConfig::find_root(tmp.path()).is_none());
}

#[test]
fn platforms_absent_is_empty() {
    let tmp = tempdir().unwrap();
    write_config(tmp.path(), "name: demo\nadapter: omnia\n");
    let cfg = ProjectConfig::load(tmp.path()).expect("loads without platforms");
    assert!(cfg.platforms.is_empty());
}

#[test]
fn platforms_field_round_trips() {
    use crate::platform::Platform;

    let tmp = tempdir().unwrap();
    write_config(
        tmp.path(),
        "name: demo\nadapter: vectis\nplatforms:\n  - core\n  - ios\n  - android\n",
    );
    let cfg = ProjectConfig::load(tmp.path()).expect("loads with platforms");
    assert_eq!(cfg.platforms, vec![Platform::Core, Platform::Ios, Platform::Android]);

    let yaml = serde_saphyr::to_string(&cfg).expect("serialise");
    assert!(yaml.contains("platforms:"), "platforms must serialise when present, got:\n{yaml}");
    assert!(yaml.contains("- core"), "must contain core, got:\n{yaml}");
    assert!(yaml.contains("- ios"), "must contain ios, got:\n{yaml}");
    assert!(yaml.contains("- android"), "must contain android, got:\n{yaml}");
}

#[test]
fn platforms_omitted_when_empty() {
    let cfg = sample_cfg(BTreeMap::new());
    let yaml = serde_saphyr::to_string(&cfg).expect("serialise");
    assert!(!yaml.contains("platforms:"), "empty platforms should be omitted, got:\n{yaml}");
}

#[test]
fn platforms_field_preserves_order() {
    use crate::platform::Platform;

    let tmp = tempdir().unwrap();
    write_config(
        tmp.path(),
        "name: demo\nadapter: vectis\nplatforms:\n  - core\n  - android\n  - ios\n  - web\n  - desktop\n",
    );
    let cfg = ProjectConfig::load(tmp.path()).expect("loads");
    assert_eq!(
        cfg.platforms,
        vec![Platform::Core, Platform::Android, Platform::Ios, Platform::Web, Platform::Desktop]
    );
}

proptest! {
    // `version_is_older` is a strict semver order: irreflexive and
    // antisymmetric over well-formed versions.
    #[test]
    fn version_order_is_strict(
        a in (0_u64..8, 0_u64..5, 0_u64..5),
        b in (0_u64..8, 0_u64..5, 0_u64..5),
    ) {
        let va = format!("{}.{}.{}", a.0, a.1, a.2);
        let vb = format!("{}.{}.{}", b.0, b.1, b.2);
        prop_assert!(!version_is_older(&va, &va));
        if version_is_older(&va, &vb) {
            prop_assert!(!version_is_older(&vb, &va));
        }
    }
}

//! Launcher-facing anchoring projection over the shared grammar: the
//! `adapter add` seed request is the one pre-boot fact argv carries.
//! Everything else — help, version, rejections, every workflow verb —
//! runs in the guest, so it projects nothing.

use transport::command::selectors::seed_request;

fn argv(args: &[&str]) -> Vec<String> {
    args.iter().map(ToString::to_string).collect()
}

#[test]
fn adapter_add_projects_seed() {
    // The seed's `--project-dir` anchors the project mount and its
    // component path earns a read-only preopen — the operator's
    // component may live outside every other mount. An absent
    // `--project-dir` projects `None` so the ancestor walk anchors
    // the deployment.
    let seed = seed_request(&argv(&["adapter", "add", "./demo.wasm"]))
        .expect("adapter add projects its seed request");
    assert_eq!(seed.component, std::path::PathBuf::from("./demo.wasm"));
    assert_eq!(seed.project_dir, None);
}

#[test]
fn adapter_add_project_dir() {
    let seed = seed_request(&argv(&["adapter", "add", "demo.wasm", "--project-dir", "/tmp/proj"]))
        .expect("seed request");
    assert_eq!(seed.project_dir, Some(std::path::PathBuf::from("/tmp/proj")));
}

#[test]
fn other_routes_project_no_seed() {
    assert_eq!(seed_request(&argv(&["plan", "status"])), None);
    assert_eq!(seed_request(&argv(&["init", "./mock.wasm"])), None);
    assert_eq!(seed_request(&argv(&["slice", "build", "s1"])), None);
}

#[test]
fn displays_and_rejections_project_no_seed() {
    // Help, version, and grammar failures all render in the guest;
    // the projection stays total and answers `None`.
    for args in [
        &["--help"][..],
        &["plan", "--help"][..],
        &["--version"][..],
        &["frobnicate"][..],
        &["plan", "transition"][..],
        &["adapter", "add"][..],
    ] {
        assert_eq!(seed_request(&argv(args)), None, "{args:?}");
    }
}

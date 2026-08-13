//! Launcher-facing anchoring projection over the shared grammar: the
//! `adapter add` seed request and the adapter refresh set are the
//! pre-boot facts argv carries. Everything else — help, version,
//! rejections, every workflow verb — runs in the guest, so it
//! projects nothing.

use transport::command::selectors::{
    ChangeRequest, RefreshRequest, change_request, refresh_request, seed_request,
};

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
fn routes_project_seed() {
    assert_eq!(seed_request(&argv(&["plan", "status"])), None);
    assert_eq!(seed_request(&argv(&["init", "./mock.wasm"])), None);
    assert_eq!(seed_request(&argv(&["slice", "build", "s1"])), None);
}

#[test]
fn displays_project_no_seed() {
    // Help, version, and grammar failures all render in the guest;
    // the projection stays total and answers `None`.
    for args in [
        &["--help"][..],
        &["plan", "--help"][..],
        &["--version"][..],
        &["frobnicate"][..],
        &["plan", "undo"][..],
        &["adapter", "add"][..],
    ] {
        assert_eq!(seed_request(&argv(args)), None, "{args:?}");
    }
}

mod refresh {
    use super::*;

    #[test]
    fn upgrade_names_the_adapter() {
        let request = refresh_request(&argv(&["adapter", "upgrade", "omnia"]));
        assert_eq!(request.names, vec!["omnia".to_string()]);
        assert!(!request.recorded_adapter);
        assert!(!request.all_bindings);
    }

    #[test]
    fn upgrade_all_flags() {
        // `--all` names nothing itself; the launcher widens the set
        // with the project's recorded bare bindings.
        let request = refresh_request(&argv(&["adapter", "upgrade", "--all"]));
        assert!(request.names.is_empty());
        assert!(request.all_bindings);
        assert_eq!(request.project_dir, None);
    }

    #[test]
    fn upgrade_all_project_dir() {
        let request =
            refresh_request(&argv(&["adapter", "upgrade", "--all", "--project-dir", "/tmp/proj"]));
        assert!(request.all_bindings);
        assert_eq!(request.project_dir, Some(std::path::PathBuf::from("/tmp/proj")));
    }

    #[test]
    fn init_bare_name_refreshes() {
        let request = refresh_request(&argv(&["init", "omnia"]));
        assert_eq!(request.names, vec!["omnia".to_string()]);
        assert!(!request.recorded_adapter);
    }

    #[test]
    fn upgrade_flags_recorded() {
        // The launcher reads `project.yaml` at the anchored root to
        // widen the set; the projection only flags the intent.
        let request = refresh_request(&argv(&["init", "--upgrade"]));
        assert!(request.names.is_empty());
        assert!(request.recorded_adapter);
    }

    #[test]
    fn pins_components_refresh() {
        // Pinned versions are immutable; local components refresh
        // through `adapter add`.
        for args in [
            &["init", "emery:omnia@1.0.0"][..],
            &["init", "./omnia.wasm"][..],
            &["adapter", "upgrade", "emery:omnia@1.0.0"][..],
        ] {
            assert!(refresh_request(&argv(args)).names.is_empty(), "{args:?}");
        }
    }

    #[test]
    fn routes_refresh_nothing() {
        for args in [
            &["plan", "status"][..],
            &["adapter", "add", "./demo.wasm"][..],
            &["slice", "build", "s1"][..],
            &["--version"][..],
            &["frobnicate"][..],
        ] {
            assert_eq!(refresh_request(&argv(args)), RefreshRequest::default(), "{args:?}");
        }
    }
}

mod change {
    use super::*;

    #[test]
    fn author_from_wave_dir() {
        let request = change_request(&argv(&[
            "plan",
            "author",
            "demo",
            "--from",
            ".emery/system/",
            "--wave",
            "w1",
            "--change-dir",
            "/tmp/change",
        ]));
        assert_eq!(
            request,
            ChangeRequest {
                change_dir: Some(std::path::PathBuf::from("/tmp/change")),
                from: Some(std::path::PathBuf::from(".emery/system/")),
            }
        );
    }

    #[test]
    fn status_change_dir() {
        let request = change_request(&argv(&["plan", "status", "--change-dir", "/tmp/change"]));
        assert_eq!(request.change_dir, Some(std::path::PathBuf::from("/tmp/change")));
        assert_eq!(request.from, None);
    }

    #[test]
    fn routes_project_nothing() {
        for args in [
            &["plan", "status"][..],
            &["adapter", "add", "./demo.wasm"][..],
            &["init", "omnia"][..],
            &["frobnicate"][..],
        ] {
            assert_eq!(change_request(&argv(args)), ChangeRequest::default(), "{args:?}");
        }
    }
}

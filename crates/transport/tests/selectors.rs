//! Launcher-facing anchoring projection over the shared grammar: the
//! adapter refresh set is the one pre-boot fact argv still carries.
//! Everything else — help, version, rejections — runs in the guest,
//! so it projects nothing.

use transport::command::selectors::{RefreshRequest, refresh_request};

fn argv(args: &[&str]) -> Vec<String> {
    args.iter().map(ToString::to_string).collect()
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
    // Pinned versions are immutable; local components are seeded at
    // init — neither joins the refresh set.
    for args in [&["init", "emery:omnia@1.0.0"][..], &["init", "./omnia.wasm"][..]] {
        assert!(refresh_request(&argv(args)).names.is_empty(), "{args:?}");
    }
}

#[test]
fn routes_refresh_nothing() {
    for args in [
        &["specify"][..],
        &["--version"][..],
        &["--help"][..],
        &["frobnicate"][..],
        &["plan", "status"][..],
    ] {
        assert_eq!(refresh_request(&argv(args)), RefreshRequest::default(), "{args:?}");
    }
}

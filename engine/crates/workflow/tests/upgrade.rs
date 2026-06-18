//! Tests for `specify_workflow::upgrade` — the pure channel classifier and
//! per-channel upgrade planner.
//!
//! The network release probe and the subprocess executor (`run_plan`) are out
//! of scope here; only the deterministic env-override branch of
//! `latest_release_tag` is exercised (no network).

use std::path::Path;

use specify_error::Error;
use specify_workflow::upgrade::{
    InstallChannel, PlannedCommand, classify, latest_release_tag, plan_upgrade,
};

const TAG_ENV: &str = "SPECIFY_RELEASE_TAG";

#[expect(unsafe_code, reason = "pin the release-tag env var for the env-override branch")]
fn set_tag_env(value: Option<&str>) {
    // SAFETY: nextest runs each test in its own process, so no sibling thread
    // reads the environment concurrently with this single mutation.
    unsafe {
        match value {
            Some(tag) => std::env::set_var(TAG_ENV, tag),
            None => std::env::remove_var(TAG_ENV),
        }
    }
}

#[test]
fn classify_cargo_home_bin() {
    let cargo_home = Path::new("/home/u/.cargo");
    let exe = Path::new("/home/u/.cargo/bin/specify");
    assert_eq!(classify(exe, Some(cargo_home)), InstallChannel::Cargo);
}

#[test]
fn classify_homebrew_cellar() {
    let exe = Path::new("/opt/homebrew/Cellar/specify/0.1.0/bin/specify");
    assert_eq!(classify(exe, None), InstallChannel::Brew);
}

#[test]
fn classify_homebrew_prefix() {
    assert_eq!(classify(Path::new("/opt/homebrew/bin/specify"), None), InstallChannel::Brew);
}

#[test]
fn classify_known_binary() {
    assert_eq!(classify(Path::new("/usr/local/bin/specify"), None), InstallChannel::Binary);
    assert_eq!(classify(Path::new("/opt/specify/specify"), None), InstallChannel::Binary);
}

#[test]
fn classify_unknown() {
    assert_eq!(classify(Path::new("/tmp/random/specify"), None), InstallChannel::Unknown);
}

#[test]
fn classify_cargo_wins_first() {
    // A cargo-home bin under /usr/local still classifies as cargo (ordered first).
    let cargo_home = Path::new("/usr/local/cargo");
    let exe = Path::new("/usr/local/cargo/bin/specify");
    assert_eq!(classify(exe, Some(cargo_home)), InstallChannel::Cargo);
}

#[test]
fn channel_wire_ids() {
    assert_eq!(InstallChannel::Cargo.as_str(), "cargo");
    assert_eq!(InstallChannel::Brew.as_str(), "brew");
    assert_eq!(InstallChannel::Binary.as_str(), "binary");
    assert_eq!(InstallChannel::Unknown.as_str(), "unknown");
}

#[test]
fn plan_cargo_pinned() {
    let plan = plan_upgrade(InstallChannel::Cargo, Some("v1.2.3")).expect("plan");
    assert!(!plan.head_fallback);
    assert_eq!(plan.commands.len(), 1);
    let cmd = &plan.commands[0];
    assert_eq!(cmd.program, "cargo");
    assert!(cmd.args.contains(&"--tag".to_string()));
    assert!(cmd.args.contains(&"v1.2.3".to_string()));
}

#[test]
fn plan_cargo_head_fallback() {
    let plan = plan_upgrade(InstallChannel::Cargo, None).expect("plan");
    assert!(plan.head_fallback, "no tag => HEAD fallback");
    assert!(!plan.commands[0].args.contains(&"--tag".to_string()));
}

#[test]
fn plan_brew() {
    let plan = plan_upgrade(InstallChannel::Brew, Some("v1.0.0")).expect("plan");
    assert_eq!(plan.commands[0].program, "brew");
    assert!(plan.guidance.is_none());
}

#[test]
fn plan_binary_guidance() {
    let plan = plan_upgrade(InstallChannel::Binary, Some("v9.9.9")).expect("plan");
    assert!(plan.commands.is_empty(), "binary channel has no shell command");
    let guidance = plan.guidance.expect("binary guidance present");
    assert!(guidance.contains("v9.9.9"), "guidance links the tagged release");
}

#[test]
fn plan_unknown_errs() {
    let err = plan_upgrade(InstallChannel::Unknown, None).expect_err("unknown has no plan");
    match err {
        Error::Diag { code, .. } => assert_eq!(code, "unknown-install-channel"),
        other => panic!("expected unknown-install-channel diagnostic, got {other:?}"),
    }
}

#[test]
fn command_display() {
    let cmd = PlannedCommand {
        program: "cargo".into(),
        args: vec!["install".into(), "--git".into()],
    };
    assert_eq!(cmd.display(), "cargo install --git");

    let bare = PlannedCommand {
        program: "brew".into(),
        args: vec![],
    };
    assert_eq!(bare.display(), "brew");
}

#[test]
fn release_tag_env_override() {
    set_tag_env(Some("v3.2.1"));
    let tag = latest_release_tag().expect("ok");
    set_tag_env(None);
    assert_eq!(tag.as_deref(), Some("v3.2.1"));
}

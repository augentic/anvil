use super::*;

#[test]
fn classify_cargo_bin() {
    let channel =
        classify(Path::new("/home/u/.cargo/bin/specify"), Some(Path::new("/home/u/.cargo")));
    assert_eq!(channel, InstallChannel::Cargo);
}

#[test]
fn classify_brew_cellar() {
    let channel = classify(
        Path::new("/opt/homebrew/Cellar/specify/0.3.0/bin/specify"),
        Some(Path::new("/home/u/.cargo")),
    );
    assert_eq!(channel, InstallChannel::Brew);
}

#[test]
fn classify_known_binary_location() {
    let channel = classify(Path::new("/usr/local/bin/specify"), Some(Path::new("/home/u/.cargo")));
    assert_eq!(channel, InstallChannel::Binary);
}

#[test]
fn classify_unknown_path() {
    let channel = classify(Path::new("/tmp/scratch/specify"), Some(Path::new("/home/u/.cargo")));
    assert_eq!(channel, InstallChannel::Unknown);
}

#[test]
fn classify_cargo_needs_home() {
    // Without a resolved CARGO_HOME a cargo-bin path cannot be proven,
    // so it falls through to Unknown rather than being misclassified.
    let channel = classify(Path::new("/home/u/.cargo/bin/specify"), None);
    assert_eq!(channel, InstallChannel::Unknown);
}

#[test]
fn tag_from_gh_payload() {
    let tag = tag_from_json(r#"{"tagName":"v0.43.0"}"#, "tagName");
    assert_eq!(tag.as_deref(), Some("v0.43.0"));
}

#[test]
fn tag_from_rest_payload() {
    let json = r#"{"tag_name":"v0.43.0","name":"0.43.0","draft":false}"#;
    assert_eq!(tag_from_json(json, "tag_name").as_deref(), Some("v0.43.0"));
}

#[test]
fn tag_from_json_rejects_missing_and_empty() {
    assert_eq!(tag_from_json(r#"{"name":"0.43.0"}"#, "tag_name"), None);
    assert_eq!(tag_from_json(r#"{"tag_name":""}"#, "tag_name"), None);
    assert_eq!(tag_from_json("not json", "tag_name"), None);
}

#[test]
fn plan_cargo_pins_tag() {
    let plan = plan_upgrade(InstallChannel::Cargo, Some("v1.2.3")).expect("cargo plan");
    assert!(!plan.head_fallback);
    assert_eq!(plan.commands.len(), 1);
    assert_eq!(plan.commands[0].program, "cargo");
    assert_eq!(plan.commands[0].args, ["install", "--git", REPO_GIT_URL, "--tag", "v1.2.3"]);
}

#[test]
fn plan_cargo_head_fallback_without_tag() {
    let plan = plan_upgrade(InstallChannel::Cargo, None).expect("cargo head plan");
    assert!(plan.head_fallback);
    assert_eq!(plan.commands[0].args, ["install", "--git", REPO_GIT_URL]);
}

#[test]
fn plan_brew_upgrades_formula() {
    let plan = plan_upgrade(InstallChannel::Brew, Some("v1.2.3")).expect("brew plan");
    assert_eq!(plan.commands[0].program, "brew");
    assert_eq!(plan.commands[0].args, ["upgrade", BREW_FORMULA]);
}

#[test]
fn plan_binary_carries_guidance_no_commands() {
    let plan = plan_upgrade(InstallChannel::Binary, Some("v1.2.3")).expect("binary plan");
    assert!(plan.commands.is_empty());
    assert!(plan.guidance.is_some());
}

#[test]
fn plan_unknown_is_diagnostic() {
    let err = plan_upgrade(InstallChannel::Unknown, None).expect_err("unknown channel errs");
    assert_eq!(err.variant_str(), "unknown-install-channel");
}

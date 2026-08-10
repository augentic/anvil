//! `project.yaml` gap-policy declaration through the public init
//! operation: fresh-init write, absent default, preservation and
//! explicit change on `--upgrade` (RFC-86a D3). The declaration is
//! inert here — the execute loop's gate consumes it in a later step.

use std::fs;

mod support;

use mock::invoke::run;
use project::GapPolicy;
use project::config::ProjectConfig;
use project::init::handlers::{Init, InitInput};
use support::{Provider, stage_cached_component};

async fn init(project: &Provider, input: InitInput) {
    run::<Init, _, _>(project, input).await.expect("init succeeds");
}

#[tokio::test]
async fn absent_means_no_declaration() {
    let project = Provider::bare();
    stage_cached_component(&project, "demo");
    init(
        &project,
        InitInput {
            adapter: Some("demo".into()),
            ..Default::default()
        },
    )
    .await;

    let cfg = ProjectConfig::load(&project.root).expect("project.yaml loads");
    assert_eq!(cfg.gap_policy, None, "no --gap-policy means no declaration (strict)");
    let raw = fs::read_to_string(project.root.join(".emery/project.yaml")).expect("raw yaml");
    assert!(!raw.contains("gap-policy"), "an absent declaration is not serialized:\n{raw}");
}

#[tokio::test]
async fn declared_written_preserved_and_changed_on_upgrade() {
    let project = Provider::bare();
    stage_cached_component(&project, "demo");
    init(
        &project,
        InitInput {
            adapter: Some("demo".into()),
            gap_policy: Some(GapPolicy::Defer),
            ..Default::default()
        },
    )
    .await;

    let raw = fs::read_to_string(project.root.join(".emery/project.yaml")).expect("raw yaml");
    assert!(raw.contains("gap-policy: defer"), "the declaration lands kebab-case:\n{raw}");
    let cfg = ProjectConfig::load(&project.root).expect("project.yaml loads");
    assert_eq!(cfg.gap_policy, Some(GapPolicy::Defer));

    // `--upgrade` without the flag preserves the recorded value,
    // like `adapter` / `description` / `rules`.
    init(
        &project,
        InitInput {
            upgrade: true,
            ..Default::default()
        },
    )
    .await;
    let cfg = ProjectConfig::load(&project.root).expect("project.yaml loads");
    assert_eq!(cfg.gap_policy, Some(GapPolicy::Defer), "upgrade preserves the declaration");

    // `--upgrade --gap-policy` updates it, mirroring `--platforms`.
    init(
        &project,
        InitInput {
            gap_policy: Some(GapPolicy::Strict),
            upgrade: true,
            ..Default::default()
        },
    )
    .await;
    let cfg = ProjectConfig::load(&project.root).expect("project.yaml loads");
    assert_eq!(cfg.gap_policy, Some(GapPolicy::Strict), "upgrade with the flag rewrites it");
}

//! Propose-kernel `target` binding and the Plan DTO hard cut, without
//! the retired survey-driven author path.

use std::collections::HashMap;

use artifacts::discovery::Discovery;
use project::adapter::catalog::Pin;
use project::plan::{Plan, ProjectRef, ProposalResponse, SourceBinding, TargetBinding};
use project::snapshot::SnapshotId;

fn topology() -> Vec<ProjectRef> {
    vec![ProjectRef {
        name: "default".into(),
        target: "mock@0.0.0".into(),
        description: None,
        surface: vec![],
        recent: vec![],
        decisions: vec![],
        decisions_more: None,
        platforms: vec![],
    }]
}

fn inventory() -> Discovery {
    Discovery::parse(
        "## Lead inventory\n\n\
         ### docs:login-flow\n\n- lead: login-flow\n- source: docs\n- synopsis: sign-in\n\n\
         ### code:login-flow\n\n- lead: login-flow\n- source: code\n- synopsis: sign-in\n\n\
         ### docs:session-timeout\n\n- lead: session-timeout\n- source: docs\n- synopsis: idle\n\n\
         ### code:session-timeout\n\n- lead: session-timeout\n- source: code\n- synopsis: idle\n\n\
         ### docs:password-reset\n\n- lead: password-reset\n- source: docs\n- synopsis: reset\n",
    )
    .expect("discovery")
}

#[test]
fn overlap_merges() {
    let response: ProposalResponse = serde_json::from_value(serde_json::json!({
        "version": 1,
        "kind": "response",
        "slices": [
            {
                "name": "login-flow",
                "target": "default",
                "sources": [
                    { "source": "docs", "lead": "login-flow" },
                    { "source": "code", "lead": "login-flow" }
                ]
            },
            {
                "name": "session-policy",
                "target": "default",
                "divergence": "likely",
                "sources": [
                    { "source": "docs", "lead": "session-timeout" },
                    { "source": "code", "lead": "session-timeout" }
                ]
            },
            {
                "name": "password-reset",
                "target": "default",
                "sources": [{ "source": "docs", "lead": "password-reset" }]
            }
        ]
    }))
    .expect("response");
    let mut plan = Plan::named("auth");
    plan.targets.insert(
        "default".into(),
        TargetBinding::new(
            Pin::parse("emery:mock@0.0.0").expect("pin"),
            ".",
            SnapshotId::from_digest(&"0".repeat(64)),
        ),
    );
    plan.sources.insert(
        "docs".into(),
        SourceBinding::intent(Pin::parse("emery:documentation@0.12.0").expect("pin"), "docs"),
    );
    plan.sources.insert(
        "code".into(),
        SourceBinding::intent(Pin::parse("emery:typescript@0.12.0").expect("pin"), "code"),
    );
    plan.propose_from(response, &inventory(), &topology(), &HashMap::new()).expect("propose");
    let login = plan.entries.iter().find(|entry| entry.name == "login-flow").expect("login");
    assert_eq!(login.target, "default");
    assert_eq!(login.sources.len(), 2);
    let session =
        plan.entries.iter().find(|entry| entry.name == "session-policy").expect("session");
    assert_eq!(session.divergence, Some(change::Divergence::Likely));
}

#[test]
fn omitted_target_refuses() {
    let err = serde_json::from_str::<ProposalResponse>(
        r#"{"version":1,"kind":"response","slices":[{"name":"only","sources":[{"source":"docs","lead":"login-flow"}]}]}"#,
    )
    .expect_err("target required");
    assert!(err.to_string().contains("target"), "{err}");
}

#[test]
fn leftover_project() {
    let yaml = "name: demo\nslices:\n  - name: a\n    target: default\n    project: default\n";
    let err = serde_saphyr::from_str::<Plan>(yaml).expect_err("unknown field");
    let text = err.to_string();
    assert!(text.contains("project") || text.contains("unknown"), "{text}");
}

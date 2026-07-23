//! Shared target-phase behavior through the public phase API.

use std::path::Path;

use adapter::Format;
use adapter::phase::{
    PhaseAnswer, assemble_system, enforce, missing_outputs, phase, render_inputs, render_outcome,
    report,
};
use adapter::seam::{
    BuildOutput, Context, Finding, Input, Platform, Report, Severity, Status, WorkingTree,
};
use omnia_testkit::model::Harness;
use tempfile::tempdir;

const fn context(root: &Path) -> Context<'_> {
    Context {
        adapter_id: "target:test",
        project_root: root,
        mcp_url: None,
    }
}

const fn success_report() -> Report {
    Report {
        status: Status::Success,
        findings: Vec::new(),
        outputs: Vec::new(),
        ui_surface: None,
    }
}

#[test]
fn tree_roots() {
    let ctx = context(Path::new("/mounted"));
    let bare = WorkingTree {
        base: "rev-1".to_string(),
        subpath: None,
    };
    let scoped = WorkingTree {
        base: "rev-1".to_string(),
        subpath: Some("project".to_string()),
    };

    assert_eq!(ctx.tree_root(&bare), Path::new("/mounted"));
    assert_eq!(ctx.tree_root(&scoped), Path::new("/mounted/project"));
}

#[tokio::test]
async fn judgment_legs() {
    let tmp = tempdir().unwrap();
    let model = Harness::answering([
        r#"{"applicable":true,"summary":"wrote core","written":["shared/src/app.rs"]}"#,
        r#"{"status":"success","findings":[]}"#,
    ]);
    let ctx = context(tmp.path());

    let answer = phase(&model, &ctx, "PHASE SYSTEM".to_string(), "PHASE USER".to_string(), "core")
        .await
        .unwrap();
    assert!(answer.applicable);
    assert_eq!(answer.summary, "wrote core");
    assert_eq!(answer.written, ["shared/src/app.rs"]);

    let result =
        report(&model, &ctx, "REPORT SYSTEM".to_string(), "REPORT USER".to_string()).await.unwrap();
    assert_eq!(result.status, Status::Success);

    let requests = model.requests();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].system.as_deref(), Some("PHASE SYSTEM"));
    assert_eq!(requests[0].messages[0].content, "PHASE USER");
    assert!(matches!(
        &requests[0].format,
        Format::Schema(schema) if schema.name == "core"
    ));
    assert!(matches!(
        &requests[1].format,
        Format::Schema(schema) if schema.name == "report"
    ));
}

#[test]
fn renderers() {
    assert_eq!(assemble_system(&["first", "second"]), "first\n\n---\n\nsecond");
    assert_eq!(render_inputs(&[]), "(no slice artifacts were provided)");
    assert_eq!(
        render_inputs(&[
            Input::Proposal("proposal body".to_string()),
            Input::Spec("spec body".to_string()),
        ]),
        "### input: proposal\n\nproposal body\n\n### input: spec\n\nspec body"
    );

    let outcome = render_outcome(
        "core",
        &PhaseAnswer {
            applicable: true,
            summary: "complete".to_string(),
            written: vec!["src/lib.rs".to_string()],
        },
    );
    assert_eq!(outcome, "- core: applicable=true, wrote [\"src/lib.rs\"] — complete");
}

#[test]
fn report_guards() {
    let tmp = tempdir().unwrap();
    std::fs::create_dir_all(tmp.path().join("present")).unwrap();

    let mut claimed = success_report();
    claimed.outputs = vec![
        BuildOutput {
            platform: Platform::Core,
            path: "present".to_string(),
        },
        BuildOutput {
            platform: Platform::Ios,
            path: "missing".to_string(),
        },
    ];
    assert_eq!(
        missing_outputs(&claimed, tmp.path()),
        ["- declared output `missing` does not exist in the working tree"]
    );

    claimed.status = Status::Failure;
    assert!(missing_outputs(&claimed, tmp.path()).is_empty());

    let residual = Finding::blocking("deterministic failure");
    let guarded = enforce(success_report(), vec![residual.clone()]);
    assert_eq!(guarded.status, Status::Failure);
    assert_eq!(guarded.findings, [residual]);

    let mut blocking_answer = success_report();
    blocking_answer.findings.push(Finding {
        rule_id: Some("TEST-001".to_string()),
        severity: Severity::Critical,
        detail: "blocking".to_string(),
    });
    assert_eq!(enforce(blocking_answer, Vec::new()).status, Status::Failure);

    let mut advisory = success_report();
    advisory.findings.push(Finding {
        rule_id: None,
        severity: Severity::Suggestion,
        detail: "advisory".to_string(),
    });
    assert_eq!(enforce(advisory, Vec::new()).status, Status::Success);
}

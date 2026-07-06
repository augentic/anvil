//! Plan-authoring orchestrator integration tests (RFC-61 Step 5,
//! Milestone S1).
//!
//! Scripted end-to-end walks of `specify_workflow::orchestrate::author`
//! against mocked Model + source seam: scaffold → survey fan-out →
//! reconcile (with Gate 1 prose) → project `plan.yaml.slices[]` →
//! persist `change.md` / `discovery.md` → validate → exit at
//! `pending`. Plus the fan-out failure surface, the repair loop over a
//! kernel-rejected grouping, and the scaffold / workspace refusals.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use jiff::Timestamp;
use serde_json::{Value, json};
use specify_workflow::change::{Lifecycle, Plan, SourceBinding, Status};
use specify_workflow::config::Layout;
use specify_workflow::orchestrate;
use specify_workflow::seam::{Lead, MockSourceSeam};
use specify_workflow::slice::SLICES_DIR_NAME;
use tempfile::TempDir;

use crate::common;

fn now() -> Timestamp {
    "2026-01-02T03:04:05Z".parse().expect("fixed timestamp parses")
}

/// A throw-away project with `.specify/project.yaml`, a stub `omnia`
/// adapter component at the development probe (so topology resolution
/// works), and a hermetic project cache. No `plan.yaml` —
/// [`orchestrate::author`] scaffolds it.
struct Project {
    _tmp: TempDir,
    _cache: common::CacheGuard,
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().to_path_buf();
        let cache = common::scoped_cache(&root);
        for sub in [format!(".specify/{SLICES_DIR_NAME}"), ".specify/specs".to_string()] {
            fs::create_dir_all(root.join(sub)).expect("mkdir");
        }
        fs::write(root.join(".specify/project.yaml"), "name: demo\nadapter: omnia\nrules: {}\n")
            .expect("write project.yaml");
        common::stage_dev_component(&root, "omnia");
        Self {
            _tmp: tmp,
            _cache: cache,
            root,
        }
    }

    fn layout(&self) -> Layout<'_> {
        Layout::new(&self.root)
    }

    fn plan(&self) -> Plan {
        Plan::load(&self.root.join("plan.yaml")).expect("load plan.yaml")
    }

    fn journal_event_ids(&self) -> Vec<String> {
        let path = self.root.join(".specify/journal.jsonl");
        if !path.exists() {
            return Vec::new();
        }
        fs::read_to_string(path)
            .expect("read journal")
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| {
                let event: Value = serde_json::from_str(l).expect("journal line is JSON");
                event["event"].as_str().expect("event id").to_string()
            })
            .collect()
    }
}

/// The two-source binding map the author verb desugars from `--source`
/// / `--intent` argv (`BTreeMap`, so survey order is `docs` then
/// `intent`).
fn bindings() -> BTreeMap<String, SourceBinding> {
    let docs: SourceBinding = serde_json::from_value(json!({
        "adapter": "documentation",
        "path": "./design-notes"
    }))
    .expect("docs binding parses");
    let intent: SourceBinding = serde_json::from_value(json!({
        "adapter": "intent",
        "value": "Refresh registration."
    }))
    .expect("intent binding parses");
    BTreeMap::from([("docs".to_string(), docs), ("intent".to_string(), intent)])
}

fn lead(id: &str, synopsis: &str) -> Lead {
    Lead {
        lead: id.to_string(),
        synopsis: synopsis.to_string(),
        topics: vec![],
    }
}

/// A schema-valid reconciliation answer: one slice matching the two
/// same-slug leads across sources, plus the Gate 1 prose sections.
fn grouping_with_gate() -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": [{
            "name": "user-registration",
            "sources": [
                { "source": "docs", "lead": "user-registration" },
                { "source": "intent", "lead": "user-registration" }
            ],
            "rationale": "Same slug and synopsis across both sources."
        }],
        "gate": {
            "change": "## Intent\n\nRefresh registration.\n\n## Scope\n\nOne slice; no tentative merges.",
            "discovery-summary": "Sources: 2. Leads: 2.",
            "discovery-source-inventory": "| key | adapter | binding |\n|---|---|---|\n| docs | documentation | ./design-notes |\n| intent | intent | \"Refresh registration.\" |"
        }
    }))
    .expect("answer serialises")
}

#[tokio::test]
async fn author_walks_to_pending_with_gate_prose() {
    let project = Project::new();
    let model =
        specify_guest_model::MockModel::answering([
            Box::leak(grouping_with_gate().into_boxed_str()) as &'static str,
        ]);
    let sources = MockSourceSeam::scripted(
        [
            Ok(vec![lead("user-registration", "Registration endpoint per the design notes.")]),
            Ok(vec![lead("user-registration", "Refresh registration.")]),
        ],
        [],
    );

    let outcome = orchestrate::author(
        &model,
        &sources,
        project.layout(),
        now(),
        "account-revamp",
        bindings(),
    )
    .await
    .expect("author walks to pending");

    // Plan at `pending` with the projected slice and structured bindings.
    let plan = project.plan();
    assert_eq!(plan.name.as_str(), "account-revamp");
    assert_eq!(plan.lifecycle, Lifecycle::Pending);
    assert_eq!(plan.entries.len(), 1);
    assert_eq!(plan.entries[0].name, "user-registration");
    assert_eq!(plan.entries[0].status, Status::Pending);
    assert_eq!(plan.entries[0].sources.len(), 2);

    // Outcome: both surveys in plan-binding order, the projected slice,
    // and the literal Gate 1 hint.
    let surveyed: Vec<(&str, &str)> = outcome
        .surveyed
        .iter()
        .map(|survey| (survey.source.as_str(), survey.adapter.as_str()))
        .collect();
    assert_eq!(surveyed, [("docs", "documentation"), ("intent", "intent")]);
    assert_eq!(outcome.slices, ["user-registration"]);
    assert!(
        outcome.hint.contains("specify plan transition account-revamp approved"),
        "the hint carries the literal Gate 1 command: {}",
        outcome.hint
    );

    // change.md: the deterministic frame plus the model's body.
    let change = fs::read_to_string(project.root.join("change.md")).expect("change.md written");
    assert!(change.starts_with("# Change — account-revamp\n\n## Intent\n"), "{change}");
    assert!(change.contains("no tentative merges"), "{change}");

    // discovery.md: the three-section preamble frames the model bodies
    // and the surveyed lead inventory rides through untouched.
    let discovery =
        fs::read_to_string(project.root.join("discovery.md")).expect("discovery.md written");
    assert!(discovery.starts_with("# Discovery — account-revamp\n\n## Summary\n"), "{discovery}");
    assert!(discovery.contains("Sources: 2. Leads: 2."), "{discovery}");
    assert!(discovery.contains("## Source inventory"), "{discovery}");
    assert!(discovery.contains("| docs | documentation | ./design-notes |"), "{discovery}");
    assert!(discovery.contains("### docs:user-registration"), "{discovery}");
    assert!(discovery.contains("### intent:user-registration"), "{discovery}");

    // The model saw the plan context (name + bindings) with the request.
    let user = &model.requests()[0].messages[0].content;
    assert!(user.contains("## Plan context"), "{user}");
    assert!(user.contains("- plan: account-revamp"), "{user}");
    assert!(user.contains("docs: adapter `documentation`, path `./design-notes`"), "{user}");
    assert!(user.contains("intent: adapter `intent`, value \"Refresh registration.\""), "{user}");

    // Journal cadence: the per-source survey pairs, then the single
    // reconcile event after the projection commits.
    assert_eq!(
        project.journal_event_ids(),
        [
            "source.execution.agent",
            "source.survey.completed",
            "source.execution.agent",
            "source.survey.completed",
            "plan.reconcile.completed",
        ]
    );
}

#[tokio::test]
async fn author_fan_out_failure_aborts() {
    let project = Project::new();
    let model = specify_guest_model::MockModel::answering([]);
    let sources = MockSourceSeam::scripted(
        [
            Ok(vec![lead("user-registration", "Registration endpoint per the design notes.")]),
            Err(specify_workflow::seam::Error::Internal("survey brief crashed".to_string())),
        ],
        [],
    );

    let err = orchestrate::author(
        &model,
        &sources,
        project.layout(),
        now(),
        "account-revamp",
        bindings(),
    )
    .await
    .expect_err("the failing survey aborts the fan-out");
    assert_eq!(err.variant_str(), "seam-dispatch-failed");
    assert!(err.to_string().contains("source:intent"), "{err}");

    // The native partial-progress posture: the scaffold and the first
    // source's merge survive; no reconcile event, no slices, no prose.
    let plan = project.plan();
    assert!(plan.entries.is_empty(), "no slices projected");
    let discovery =
        fs::read_to_string(project.root.join("discovery.md")).expect("first source merged");
    assert!(discovery.contains("### docs:user-registration"), "{discovery}");
    assert!(!project.root.join("change.md").exists(), "no Gate 1 prose on failure");
    assert_eq!(
        project.journal_event_ids(),
        ["source.execution.agent", "source.survey.completed", "source.execution.agent"]
    );
}

#[tokio::test]
async fn author_repairs_kernel_rejected_grouping() {
    let project = Project::new();
    // First answer misses the `intent` lead (coverage gap → kernel
    // rejection in the check) and carries no gate; the repair succeeds.
    let uncovered = serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": [{
            "name": "user-registration",
            "sources": [{ "source": "docs", "lead": "user-registration" }]
        }]
    }))
    .expect("answer serialises");
    let model = specify_guest_model::MockModel::answering([
        Box::leak(uncovered.into_boxed_str()) as &'static str,
        Box::leak(grouping_with_gate().into_boxed_str()) as &'static str,
    ]);
    let sources = MockSourceSeam::scripted(
        [
            Ok(vec![lead("user-registration", "Registration endpoint per the design notes.")]),
            Ok(vec![lead("user-registration", "Refresh registration.")]),
        ],
        [],
    );

    let outcome = orchestrate::author(
        &model,
        &sources,
        project.layout(),
        now(),
        "account-revamp",
        bindings(),
    )
    .await
    .expect("the kernel-rejected grouping repairs in-loop");
    assert_eq!(outcome.slices, ["user-registration"]);

    let calls = model.requests();
    assert_eq!(calls.len(), 2, "one repair attempt");
    let repair = &calls[1].messages[0].content;
    assert!(repair.contains("## Findings"), "repair prompt carries the findings: {repair}");
    assert!(repair.contains("Previous answer"), "{repair}");

    // The plan carries the repaired grouping, not the rejected one.
    let plan = project.plan();
    assert_eq!(plan.entries[0].sources.len(), 2);
}

#[tokio::test]
async fn author_gate_missing_exhausts_budget() {
    let project = Project::new();
    // Kernel-valid grouping, but no gate prose — the check refuses it
    // every attempt until the budget exhausts.
    let gateless = serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slices": [{
            "name": "user-registration",
            "sources": [
                { "source": "docs", "lead": "user-registration" },
                { "source": "intent", "lead": "user-registration" }
            ]
        }]
    }))
    .expect("answer serialises");
    let answer = Box::leak(gateless.into_boxed_str()) as &'static str;
    let model = specify_guest_model::MockModel::answering([answer, answer, answer]);
    let sources = MockSourceSeam::scripted(
        [
            Ok(vec![lead("user-registration", "Registration endpoint per the design notes.")]),
            Ok(vec![lead("user-registration", "Refresh registration.")]),
        ],
        [],
    );

    let err = orchestrate::author(
        &model,
        &sources,
        project.layout(),
        now(),
        "account-revamp",
        bindings(),
    )
    .await
    .expect_err("gate-less answers exhaust the repair budget");
    assert_eq!(err.variant_str(), "plan-author-gate-missing");
    assert_eq!(model.requests().len(), 3, "initial attempt plus MAX_REPAIRS");

    // No projection committed, no reconcile event.
    assert!(project.plan().entries.is_empty());
    assert!(!project.journal_event_ids().contains(&"plan.reconcile.completed".to_string()));
}

#[tokio::test]
async fn author_refuses_existing_plan() {
    let project = Project::new();
    fs::write(project.root.join("plan.yaml"), "name: other\nlifecycle: pending\nslices: []\n")
        .expect("pre-seed plan.yaml");

    let model = specify_guest_model::MockModel::answering([]);
    let sources = MockSourceSeam::scripted([], []);
    let err = orchestrate::author(
        &model,
        &sources,
        project.layout(),
        now(),
        "account-revamp",
        bindings(),
    )
    .await
    .expect_err("an existing plan refuses the scaffold");
    assert_eq!(err.variant_str(), "already-exists");
    assert!(project.journal_event_ids().is_empty(), "refused before any survey");
}

#[tokio::test]
async fn author_refuses_workspace_root() {
    let project = Project::new();
    fs::write(
        project.root.join(".specify/project.yaml"),
        "name: demo\nworkspace: true\nrules: {}\n",
    )
    .expect("workspace project.yaml");

    let model = specify_guest_model::MockModel::answering([]);
    let sources = MockSourceSeam::scripted([], []);
    let err = orchestrate::author(
        &model,
        &sources,
        project.layout(),
        now(),
        "account-revamp",
        bindings(),
    )
    .await
    .expect_err("a workspace root refuses the collapse");
    assert_eq!(err.variant_str(), "plan-author-workspace-unsupported");
    assert!(!project.root.join("plan.yaml").exists(), "refused before the scaffold");
}

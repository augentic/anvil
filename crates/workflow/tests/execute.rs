//! Guest execute-loop integration tests.
//!
//! Scripted end-to-end walk of `workflow::orchestrate::execute`
//! against mocked Model + seams: plan approved → claim → refine (extract
//! fan-out, synthesis judgment, persist, validate, `refined`) → build →
//! merge, per entry, to `drained`. Plus the typed stop paths (Gate 1
//! refusal, a failing build), the create-exclusive guest-marker
//! posture, and the standalone `slice refine <name>` breakout
//! (`orchestrate::refine_breakout`), which shares this harness.

use std::fs;
use std::path::PathBuf;

use jiff::Timestamp;
use serde_json::{Value, json};
use tempfile::TempDir;
use workflow::change::{LoopStep, Plan, Status, StopReason};
use workflow::config::Layout;
use workflow::orchestrate::{self, ExecuteOutcome};
use workflow::seam::{Evidence, Lead, MockSourceSeam, MockTargetSeam, WorkingTree};
use workflow::slice::{BuildReport, BuildStatus, SLICES_DIR_NAME, UiSurface};

mod common;

fn now() -> Timestamp {
    "2026-01-02T03:04:05Z".parse().expect("fixed timestamp parses")
}

/// A throw-away project with `.specify/project.yaml`, a stub `omnia`
/// adapter component at the development probe (so topology / target
/// resolution works), and a hermetic project cache.
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
        // Stage the omnia adapter component at the in-repo development
        // probe so `TargetAdapter::resolve` (topology / plan-next
        // target resolution) succeeds locally.
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

    fn slices_dir(&self) -> PathBuf {
        self.root.join(".specify").join(SLICES_DIR_NAME)
    }

    fn seed_plan(&self, content: &str) {
        fs::write(self.root.join("plan.yaml"), content).expect("write plan.yaml");
    }

    fn plan(&self) -> Plan {
        Plan::load(&self.root.join("plan.yaml")).expect("load plan.yaml")
    }

    fn journal(&self) -> Vec<Value> {
        let path = self.root.join(".specify/journal.jsonl");
        if !path.exists() {
            return Vec::new();
        }
        fs::read_to_string(path)
            .expect("read journal")
            .lines()
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_str(l).expect("journal line is JSON"))
            .collect()
    }

    fn journal_event_ids(&self) -> Vec<String> {
        self.journal().iter().map(|e| e["event"].as_str().expect("event id").to_string()).collect()
    }

    fn marker_path(&self) -> PathBuf {
        self.root.join(".specify/guest.lock")
    }

    /// Seed `discovery.md` with the surveyed intent leads so extract can
    /// resolve each slice's `(source, lead)` pair.
    async fn seed_discovery(&self, leads: &[&str]) {
        // One survey dispatch per plan source binding; the single
        // `intent` binding returns every lead at once.
        let all: Vec<Lead> = leads
            .iter()
            .map(|id| Lead {
                lead: (*id).to_string(),
                synopsis: format!("Operator intent for {id}."),
                topics: vec!["intent".to_string()],
            })
            .collect();
        let seam = MockSourceSeam::scripted([Ok(all)], []);
        orchestrate::survey_all(&seam, self.layout(), now()).await.expect("seed survey");
    }
}

const APPROVED_PLAN: &str = "\
name: demo
lifecycle: approved
sources:
  intent:
    adapter: intent
    value: \"Build the demo.\"
slices:
  - name: feature-x
    status: pending
    sources:
      - { source: intent, lead: feature-x }
  - name: feature-y
    status: pending
    depends-on: [feature-x]
    sources:
      - { source: intent, lead: feature-y }
";

/// Intent Evidence for one slice: a single id-bearing `kind: intent`
/// claim the response anchors.
fn intent_evidence(claim_id: &str, statement: &str) -> Evidence {
    Evidence {
        authority: artifacts::evidence::AuthorityClass::Intent,
        claims: vec![json!({
            "kind": "intent",
            "id": claim_id,
            "statement": statement,
        })],
    }
}

/// A schema-valid synthesis response for one slice: one requirement
/// citing the intent claim (with a scenario, so the adapter rules
/// pass), a satisfying task, and artifacts carrying the proposal
/// sections `slice validate` requires.
///
/// `req_id` is the requirement id the kernel will allocate — REQ ids
/// are baseline-aware, so a slice refined after another slice merged
/// gets the next free id (the inputs envelope's baseline detail tells
/// a real agent the same thing).
fn synthesis_response(slice: &str, domain: &str, claim_id: &str, req_id: &str) -> String {
    serde_json::to_string(&json!({
        "version": 1,
        "kind": "response",
        "slice": slice,
        "model": {
            "requirements": [{
                "title": format!("{domain} behaves as intended"),
                "domain": domain,
                "claims": [{ "source": "intent", "id": claim_id, "kind": "intent" }],
                "statement": format!("The {domain} surface behaves as the operator intends."),
                "scenarios": ["Intended behaviour observed"]
            }],
            "tasks": [
                { "id": "TASK-001", "text": format!("Implement the {domain} change."), "satisfies": [req_id] }
            ]
        },
        "artifacts": {
            "proposal": format!("# {slice}\n\n## Why\n\nThe operator asked for it.\n\n## Domains\n\n- {domain} — the affected surface\n\n## Non-goals\n\n- Nothing else.\n"),
            "design": format!("# Design\n\nHow {slice} lands.\n"),
            "tasks": "# Tasks\n\n## Implementation\n\n- [ ] 1.1 Implement the change (TASK-001)\n",
            "specs": [{ "domain": domain, "content": format!("## {domain}\nAgent prose body.\n") }]
        }
    }))
    .expect("response serialises")
}

fn success_report(slice: &str) -> BuildReport {
    BuildReport {
        version: 1,
        slice: slice.to_string(),
        target: "omnia@1.0.0".to_string(),
        status: BuildStatus::Success,
        findings: vec![],
        outputs: vec![],
        ui_surface: Some(UiSurface { screens: 0 }),
    }
}

fn failure_report(slice: &str) -> BuildReport {
    BuildReport {
        status: BuildStatus::Failure,
        ..success_report(slice)
    }
}

fn tree() -> WorkingTree {
    WorkingTree {
        base: "live".to_string(),
        subpath: None,
    }
}

/// The per-slice journal cadence one full claim → refine → build →
/// merge pass composes.
const SLICE_CADENCE: [&str; 15] = [
    "plan.entry.advanced",
    "source.execution.agent",
    "slice.extract.completed",
    "slice.synthesize.agent",
    "slice.synthesize.started",
    "slice.synthesize.completed",
    "slice.transition.refined",
    "target.execution.agent",
    "slice.build.started",
    "slice.build.succeeded",
    "slice.merge.started",
    "slice.merge.commit-skipped",
    "slice.archive.created",
    "slice.merge.succeeded",
    // `plan next` for the following claim window emits nothing extra;
    // the merge stamped `done` so the next advance opens the next
    // slice's cadence.
    "",
];

#[tokio::test]
async fn execute_drains_two_entry_plan() {
    let project = Project::new();
    project.seed_plan(APPROVED_PLAN);
    project.seed_discovery(&["feature-x", "feature-y"]).await;
    let survey_events = project.journal_event_ids().len();

    let model = testkit::MockModel::answering([
        Box::leak(
            synthesis_response("feature-x", "greeting", "greeting.fix", "REQ-001").into_boxed_str(),
        ) as &'static str,
        Box::leak(
            synthesis_response("feature-y", "billing", "billing.fix", "REQ-002").into_boxed_str(),
        ) as &'static str,
    ]);
    let sources = MockSourceSeam::scripted(
        [],
        [
            Ok(intent_evidence("greeting.fix", "Fix the greeting.")),
            Ok(intent_evidence("billing.fix", "Fix the billing export.")),
        ],
    );
    let targets = MockTargetSeam::scripted(
        [Ok("Shape guidance.".to_string()), Ok("Shape guidance.".to_string())],
        [Ok(success_report("feature-x")), Ok(success_report("feature-y"))],
    );

    let outcome =
        orchestrate::execute(&model, &sources, &targets, project.layout(), now(), &[], &tree())
            .await
            .expect("execute drains the plan");

    // Drained, with every phase in claim order.
    let ExecuteOutcome::Drained { phases } = outcome else {
        panic!("expected a drained outcome, got {outcome:?}");
    };
    let ran: Vec<(&str, LoopStep)> =
        phases.iter().map(|run| (run.slice.as_str(), run.step)).collect();
    assert_eq!(
        ran,
        [
            ("feature-x", LoopStep::Refine),
            ("feature-x", LoopStep::Build),
            ("feature-x", LoopStep::Merge),
            ("feature-y", LoopStep::Refine),
            ("feature-y", LoopStep::Build),
            ("feature-y", LoopStep::Merge),
        ]
    );

    // Every plan entry stamped `done` (merge is the only done writer).
    let plan = project.plan();
    assert!(plan.entries.iter().all(|e| e.status == Status::Done), "{:?}", plan.entries);

    // Slice trees archived; baseline specs written per domain.
    for (slice, domain, req_id) in
        [("feature-x", "greeting", "REQ-001"), ("feature-y", "billing", "REQ-002")]
    {
        assert!(!project.slices_dir().join(slice).exists(), "{slice} archived");
        assert!(
            project.root.join(".specify/archive").join(format!("2026-01-02-{slice}")).is_dir(),
            "{slice} archive dir exists"
        );
        let baseline = project.root.join(".specify/specs").join(domain).join("spec.md");
        let content = fs::read_to_string(&baseline).expect("baseline spec written");
        assert!(content.contains(&format!("ID: {req_id}")), "{content}");
        assert!(content.contains("Sources: intent"), "{content}");
    }
    // Archived artifact set persisted by the synthesis tail.
    let archived = project.root.join(".specify/archive/2026-01-02-feature-x");
    for artifact in ["proposal.md", "design.md", "tasks.md", "model.yaml", "metadata.yaml"] {
        assert!(archived.join(artifact).is_file(), "{artifact} persisted");
    }

    // Journal cadence: the full per-slice composition, twice, after the
    // seed survey events. No plan.execute.* events exist.
    let ids = project.journal_event_ids();
    let expected: Vec<&str> = SLICE_CADENCE
        .iter()
        .chain(SLICE_CADENCE.iter())
        .copied()
        .filter(|id| !id.is_empty())
        .collect();
    assert_eq!(&ids[survey_events..], expected.as_slice());

    // Skipped-git posture coherent at drained: no git side effects, no
    // merge-sha on either ledger entry, the skip recorded explicitly.
    assert!(!project.root.join(".git").exists(), "the guest run must not create a repository");
    for event in project.journal() {
        if event["event"] == "slice.archive.created" {
            assert!(
                event["payload"].get("merge-sha").is_none(),
                "guest ledger entries carry no merge-sha: {event}"
            );
        }
    }

    // D1 marker released on the clean exit.
    assert!(!project.marker_path().exists(), "marker removed on drop");
}

#[tokio::test]
async fn execute_refuses_unapproved_plan() {
    let project = Project::new();
    project.seed_plan(&APPROVED_PLAN.replace("lifecycle: approved\n", ""));

    let model = testkit::MockModel::answering([]);
    let sources = MockSourceSeam::scripted([], []);
    let targets = MockTargetSeam::scripted([], []);

    let outcome =
        orchestrate::execute(&model, &sources, &targets, project.layout(), now(), &[], &tree())
            .await
            .expect("gate-1 refusal is a typed stop, not an error");
    let ExecuteOutcome::Stopped {
        reason, hint, phases, ..
    } = outcome
    else {
        panic!("expected a stop outcome, got {outcome:?}");
    };
    assert_eq!(reason, StopReason::PlanNotApproved);
    assert!(hint.contains("plan transition"), "{hint}");
    assert!(phases.is_empty(), "no phase runs before the gate");

    // Nothing claimed, nothing journalled, marker released.
    let plan = project.plan();
    assert!(plan.entries.iter().all(|e| e.status == Status::Pending));
    assert!(project.journal_event_ids().is_empty());
    assert!(!project.marker_path().exists());
}

#[tokio::test]
async fn build_failure_stops_typed_entry_kept() {
    let project = Project::new();
    project.seed_plan(APPROVED_PLAN);
    project.seed_discovery(&["feature-x", "feature-y"]).await;

    let model = testkit::MockModel::answering([Box::leak(
        synthesis_response("feature-x", "greeting", "greeting.fix", "REQ-001").into_boxed_str(),
    ) as &'static str]);
    let sources =
        MockSourceSeam::scripted([], [Ok(intent_evidence("greeting.fix", "Fix the greeting."))]);
    let targets = MockTargetSeam::scripted(
        [Ok("Shape guidance.".to_string())],
        [Ok(failure_report("feature-x"))],
    );

    let outcome =
        orchestrate::execute(&model, &sources, &targets, project.layout(), now(), &[], &tree())
            .await
            .expect("a failing phase is a typed stop, not an error");
    let ExecuteOutcome::Stopped {
        reason,
        detail,
        slice,
        phases,
        ..
    } = outcome
    else {
        panic!("expected a stop outcome, got {outcome:?}");
    };
    assert_eq!(reason, StopReason::BuildFailed);
    assert_eq!(slice.as_deref(), Some("feature-x"));
    assert!(detail.expect("failure detail").contains("failed build"), "detail carries the cause");
    assert_eq!(phases.len(), 1, "refine completed before the build failed");
    assert_eq!(phases[0].step, LoopStep::Refine);

    // The entry stays in-progress and the failure terminal is
    // journalled, so a re-entrant run's status projection re-reports
    // the same stop.
    let plan = project.plan();
    let entry = plan.entries.iter().find(|e| e.name == "feature-x").expect("entry");
    assert_eq!(entry.status, Status::InProgress);
    let ids = project.journal_event_ids();
    assert_eq!(ids.last().map(String::as_str), Some("slice.build.failed"));
    assert!(!project.marker_path().exists(), "marker released on the stop return");

    // Re-entry: a fresh execute (no phases scripted — nothing should
    // dispatch) re-reports the same typed stop from the journal
    // overlay.
    let model = testkit::MockModel::answering([]);
    let sources = MockSourceSeam::scripted([], []);
    let targets = MockTargetSeam::scripted([], []);
    let outcome =
        orchestrate::execute(&model, &sources, &targets, project.layout(), now(), &[], &tree())
            .await
            .expect("re-entry is safe");
    let ExecuteOutcome::Stopped { reason, .. } = outcome else {
        panic!("expected the re-reported stop, got {outcome:?}");
    };
    assert_eq!(reason, StopReason::BuildFailed);
}

/// The refine breakout acts on the named slice directly against a
/// `pending` entry — it refines to `refined` without advancing
/// per-entry status (`plan next` stays the only `in-progress` writer).
#[tokio::test]
async fn refine_breakout_skips_entry_claim() {
    let project = Project::new();
    project.seed_plan(APPROVED_PLAN);
    project.seed_discovery(&["feature-x", "feature-y"]).await;
    let survey_events = project.journal_event_ids().len();

    let model = testkit::MockModel::answering([Box::leak(
        synthesis_response("feature-x", "greeting", "greeting.fix", "REQ-001").into_boxed_str(),
    ) as &'static str]);
    let sources =
        MockSourceSeam::scripted([], [Ok(intent_evidence("greeting.fix", "Fix the greeting."))]);
    let targets = MockTargetSeam::scripted([Ok("Shape guidance.".to_string())], []);

    let outcome = orchestrate::refine_breakout(
        &model,
        &sources,
        &targets,
        project.layout(),
        now(),
        "feature-x",
    )
    .await
    .expect("the breakout refines the named slice");
    assert_eq!(outcome.slice, "feature-x");
    assert_eq!(outcome.extracted, [("intent".to_string(), "feature-x".to_string())]);

    // The slice is `refined`; the plan entry was never claimed.
    let metadata = workflow::slice::SliceMetadata::load(&project.slices_dir().join("feature-x"))
        .expect("slice metadata");
    assert_eq!(metadata.status, workflow::slice::LifecycleStatus::Refined);
    // Development (unpinned) components resolve as the honest `0.0.0`
    // placeholder — there is no published package identity to record.
    assert_eq!(metadata.target, "omnia@0.0.0", "target resolved from the bound topology");
    let plan = project.plan();
    let entry = plan.entries.iter().find(|e| e.name == "feature-x").expect("entry");
    assert_eq!(entry.status, Status::Pending, "the breakout never advances per-entry status");

    // The per-phase refine cadence, with no `plan.entry.advanced`.
    let ids = project.journal_event_ids();
    assert_eq!(
        &ids[survey_events..],
        [
            "source.execution.agent",
            "slice.extract.completed",
            "slice.synthesize.agent",
            "slice.synthesize.started",
            "slice.synthesize.completed",
            "slice.transition.refined",
        ]
    );
}

/// A `done` entry refuses the breakout — merge already folded the
/// slice into the baseline.
#[tokio::test]
async fn refine_breakout_refuses_done_entry() {
    let project = Project::new();
    project.seed_plan(&APPROVED_PLAN.replace(
        "  - name: feature-x\n    status: pending\n",
        "  - name: feature-x\n    status: done\n",
    ));

    let model = testkit::MockModel::answering([]);
    let sources = MockSourceSeam::scripted([], []);
    let targets = MockTargetSeam::scripted([], []);
    let err = orchestrate::refine_breakout(
        &model,
        &sources,
        &targets,
        project.layout(),
        now(),
        "feature-x",
    )
    .await
    .expect_err("a done entry refuses the breakout");
    assert_eq!(err.variant_str(), "slice-refine-entry-done");
    assert!(err.to_string().contains("--undo"), "the error names the walk-back verb: {err}");
    assert!(project.journal_event_ids().is_empty(), "refused before any phase work");
}

/// An unknown slice name surfaces the same entry-missing error the
/// refine phase raises inside the loop.
#[tokio::test]
async fn refine_breakout_refuses_unknown_entry() {
    let project = Project::new();
    project.seed_plan(APPROVED_PLAN);

    let model = testkit::MockModel::answering([]);
    let sources = MockSourceSeam::scripted([], []);
    let targets = MockTargetSeam::scripted([], []);
    let err = orchestrate::refine_breakout(
        &model,
        &sources,
        &targets,
        project.layout(),
        now(),
        "feature-z",
    )
    .await
    .expect_err("an unknown entry refuses the breakout");
    assert_eq!(err.variant_str(), "slice-synthesize-entry-missing");
}

#[tokio::test]
async fn workspace_routed_plan_refused() {
    let project = Project::new();
    // A `project`-scoped entry means the skill would slot-route it;
    // the guest loop refuses instead of writing to the wrong tree.
    project.seed_plan(
        &APPROVED_PLAN
            .replace("  - name: feature-x\n", "  - name: feature-x\n    project: demo-app\n"),
    );

    let model = testkit::MockModel::answering([]);
    let sources = MockSourceSeam::scripted([], []);
    let targets = MockTargetSeam::scripted([], []);
    let err =
        orchestrate::execute(&model, &sources, &targets, project.layout(), now(), &[], &tree())
            .await
            .expect_err("a workspace-routed plan refuses the run");
    assert_eq!(err.variant_str(), "plan-execute-workspace-unsupported");
    assert!(err.to_string().contains("demo-app"), "the error names the scoped project: {err}");

    // Refused before the marker and before any plan state.
    assert!(!project.marker_path().exists());
    assert!(project.journal_event_ids().is_empty());
    let plan = project.plan();
    assert!(plan.entries.iter().all(|e| e.status == Status::Pending));
}

#[tokio::test]
async fn held_marker_refused_and_named() {
    let project = Project::new();
    project.seed_plan(APPROVED_PLAN);

    // A live (or crash-stale) marker refuses the run before any plan
    // state is touched — the operator deletes the named file to
    // recover.
    fs::write(project.marker_path(), "pid=999\nhostname=other\n").expect("pre-create marker");

    let model = testkit::MockModel::answering([]);
    let sources = MockSourceSeam::scripted([], []);
    let targets = MockTargetSeam::scripted([], []);
    let err =
        orchestrate::execute(&model, &sources, &targets, project.layout(), now(), &[], &tree())
            .await
            .expect_err("a held marker refuses the run");
    assert_eq!(err.variant_str(), "guest-marker-held");
    assert!(err.to_string().contains("guest.lock"), "the error names the marker file: {err}");
    assert!(err.to_string().contains("delete the file"), "{err}");

    // The refusal must not delete the holder's marker.
    assert!(project.marker_path().exists(), "a failed acquire leaves the marker in place");
    assert!(project.journal_event_ids().is_empty(), "no plan state touched");
}

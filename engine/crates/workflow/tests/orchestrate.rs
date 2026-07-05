//! Guest orchestrator integration tests (RFC-61 Step 4, Milestone C).
//!
//! Each test builds a throw-away project under `tempfile::TempDir` and
//! drives the `specify_workflow::orchestrate` functions against the
//! scripted seam mocks, proving the fan-out, the validate-before-visible
//! tails, and the journal cadence match the native verbs.

use std::fs;
use std::path::{Path, PathBuf};

use jiff::Timestamp;
use serde_json::{Value, json};
use specify_diagnostics::{
    Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity as DiagSeverity,
};
use specify_workflow::config::Layout;
use specify_workflow::merge::{ArtifactClass, MergeStrategy};
use specify_workflow::orchestrate;
use specify_workflow::seam::{
    Error as SeamError, Evidence, Lead, MockSourceSeam, MockTargetSeam, SourceCall, TargetCall,
    WorkingTree,
};
use specify_workflow::slice::{
    BuildOutput, BuildReport, BuildStatus, LifecycleStatus, SLICES_DIR_NAME, SliceMetadata,
    UiSurface,
};
use tempfile::TempDir;

const SLICE_NAME: &str = "feature-x";

fn now() -> Timestamp {
    "2026-01-02T03:04:05Z".parse().expect("fixed timestamp parses")
}

struct Project {
    _tmp: TempDir,
    root: PathBuf,
}

impl Project {
    fn new() -> Self {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().to_path_buf();
        for sub in [
            format!(".specify/{SLICES_DIR_NAME}"),
            ".specify/specs".to_string(),
            ".specify/archive".to_string(),
        ] {
            fs::create_dir_all(root.join(sub)).expect("mkdir");
        }
        Self { _tmp: tmp, root }
    }

    fn layout(&self) -> Layout<'_> {
        Layout::new(&self.root)
    }

    fn slice_dir(&self) -> PathBuf {
        self.root.join(".specify").join(SLICES_DIR_NAME).join(SLICE_NAME)
    }

    fn seed_plan(&self, content: &str) {
        fs::write(self.root.join("plan.yaml"), content).expect("write plan.yaml");
    }

    /// Journal lines as parsed JSON values, empty when no journal
    /// exists.
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
}

const TWO_SOURCE_PLAN: &str = "\
name: demo
sources:
  docs:
    adapter: documentation
    path: ./docs
  legacy:
    adapter: typescript
    path: ./legacy
slices:
  - name: feature-x
    status: pending
    sources:
      - { source: legacy, lead: user-registration }
";

fn lead(id: &str, synopsis: &str) -> Lead {
    Lead {
        lead: id.to_string(),
        synopsis: synopsis.to_string(),
        topics: vec!["identity".to_string()],
    }
}

// ---------------------------------------------------------------------------
// survey_all
// ---------------------------------------------------------------------------

#[tokio::test]
async fn survey_all_fans_out_and_merges_discovery() {
    let project = Project::new();
    project.seed_plan(TWO_SOURCE_PLAN);

    // Bindings iterate in plan-key order (BTreeMap): docs then legacy.
    let seam = MockSourceSeam::scripted(
        [
            Ok(vec![lead("billing-export", "Docs describe a billing export.")]),
            Ok(vec![lead("user-registration", "Registration endpoint in the monolith.")]),
        ],
        [],
    );

    let surveyed = orchestrate::survey_all(&seam, project.layout(), now())
        .await
        .expect("survey fan-out succeeds");
    assert_eq!(surveyed.len(), 2);
    assert_eq!(surveyed[0].source, "docs");
    assert_eq!(surveyed[0].adapter, "documentation");
    assert_eq!(surveyed[0].leads, ["billing-export"]);
    assert_eq!(surveyed[1].source, "legacy");
    assert_eq!(surveyed[1].leads, ["user-registration"]);

    // Dispatches routed by the plan-bound adapter id.
    assert_eq!(
        seam.calls(),
        [
            SourceCall::Survey {
                id: "source:documentation".to_string()
            },
            SourceCall::Survey {
                id: "source:typescript".to_string()
            },
        ]
    );

    // Leads merged into discovery.md with orchestrator-stamped sources.
    let discovery = fs::read_to_string(project.root.join("discovery.md")).expect("discovery.md");
    assert!(discovery.contains("billing-export"), "{discovery}");
    assert!(discovery.contains("source: docs"), "{discovery}");
    assert!(discovery.contains("user-registration"), "{discovery}");
    assert!(discovery.contains("source: legacy"), "{discovery}");

    // Journal cadence mirrors the native verb: execution.agent then
    // survey.completed, per source.
    assert_eq!(
        project.journal_event_ids(),
        [
            "source.execution.agent",
            "source.survey.completed",
            "source.execution.agent",
            "source.survey.completed",
        ]
    );
    let events = project.journal();
    assert_eq!(events[0]["payload"]["operation"], "survey");
    assert_eq!(events[1]["payload"]["source"], "docs");
    assert_eq!(events[1]["payload"]["adapter"], "documentation");
}

#[tokio::test]
async fn survey_schema_gate_blocks_merge() {
    let project = Project::new();
    project.seed_plan(TWO_SOURCE_PLAN);

    // An invalid lead id (spaces, uppercase) fails the schema gate
    // before anything reaches discovery.md.
    let seam = MockSourceSeam::scripted([Ok(vec![lead("Not Kebab", "Broken lead.")])], []);

    let err = orchestrate::survey_all(&seam, project.layout(), now())
        .await
        .expect_err("invalid lead set is rejected");
    assert!(err.to_string().contains("lead"), "{err}");
    assert!(
        !project.root.join("discovery.md").exists(),
        "an invalid lead set must leave discovery.md unwritten"
    );
}

#[tokio::test]
async fn survey_seam_failure_maps_to_wire_code() {
    let project = Project::new();
    project.seed_plan(TWO_SOURCE_PLAN);
    let seam =
        MockSourceSeam::scripted([Err(SeamError::Internal("model unavailable".to_string()))], []);

    let err = orchestrate::survey_all(&seam, project.layout(), now())
        .await
        .expect_err("seam failure propagates");
    assert_eq!(err.variant_str(), "seam-dispatch-failed");
    assert!(err.to_string().contains("source:documentation"), "{err}");
}

// ---------------------------------------------------------------------------
// extract
// ---------------------------------------------------------------------------

/// Survey `legacy` so `discovery.md` carries the lead extract resolves.
async fn seed_discovery(project: &Project) {
    let seam = MockSourceSeam::scripted(
        [
            Ok(vec![lead("billing-export", "Docs describe a billing export.")]),
            Ok(vec![lead("user-registration", "Registration endpoint in the monolith.")]),
        ],
        [],
    );
    orchestrate::survey_all(&seam, project.layout(), now()).await.expect("seed survey");
}

fn evidence() -> Evidence {
    Evidence {
        authority: specify_model::evidence::AuthorityClass::Behaviour,
        claims: vec![json!({
            "kind": "requirement",
            "id": "users.register",
            "path": "src/users.ts#L10-L42",
            "statement": "Registrations require an RFC 5322 email."
        })],
    }
}

#[tokio::test]
async fn extract_persists_schema_gated_evidence() {
    let project = Project::new();
    project.seed_plan(TWO_SOURCE_PLAN);
    seed_discovery(&project).await;

    let seam = MockSourceSeam::scripted([], [Ok(evidence())]);
    let outcome = orchestrate::extract(
        &seam,
        project.layout(),
        now(),
        "legacy",
        "user-registration",
        SLICE_NAME,
    )
    .await
    .expect("extract succeeds");

    assert_eq!(outcome.source, "legacy");
    assert_eq!(outcome.adapter, "typescript");
    let expected_path = project.slice_dir().join("evidence/legacy.yaml");
    assert_eq!(outcome.evidence, expected_path);

    // The seam received the discovery-resolved lead, source-less per
    // the WIT shape.
    assert_eq!(
        seam.calls(),
        [SourceCall::Extract {
            id: "source:typescript".to_string(),
            lead: lead("user-registration", "Registration endpoint in the monolith."),
        }]
    );

    // The persisted document rejoins the envelope `lead` key and keeps
    // the open claim body fields verbatim.
    let persisted = fs::read_to_string(&expected_path).expect("evidence persisted");
    assert!(persisted.contains("lead: user-registration"), "{persisted}");
    assert!(persisted.contains("authority: behaviour"), "{persisted}");
    assert!(persisted.contains("users.register"), "{persisted}");
    assert!(persisted.contains("RFC 5322"), "{persisted}");

    let ids = project.journal_event_ids();
    assert_eq!(
        &ids[ids.len() - 2..],
        ["source.execution.agent", "slice.extract.completed"],
        "extract journals the handoff then the completion"
    );
}

#[tokio::test]
async fn extract_unknown_lead_is_rejected() {
    let project = Project::new();
    project.seed_plan(TWO_SOURCE_PLAN);
    seed_discovery(&project).await;

    let seam = MockSourceSeam::scripted([], [Ok(evidence())]);
    let err =
        orchestrate::extract(&seam, project.layout(), now(), "legacy", "no-such-lead", SLICE_NAME)
            .await
            .expect_err("unknown lead is rejected");
    assert_eq!(err.variant_str(), "discovery-lead-unknown");
    assert!(seam.calls().is_empty(), "no dispatch for an unresolvable lead");
    assert!(!project.slice_dir().join("evidence/legacy.yaml").exists());
}

#[tokio::test]
async fn extract_unknown_source_is_rejected() {
    let project = Project::new();
    project.seed_plan(TWO_SOURCE_PLAN);

    let seam = MockSourceSeam::scripted([], []);
    let err = orchestrate::extract(
        &seam,
        project.layout(),
        now(),
        "nope",
        "user-registration",
        SLICE_NAME,
    )
    .await
    .expect_err("unknown source is rejected");
    assert_eq!(err.variant_str(), "source-unknown");
}

// ---------------------------------------------------------------------------
// build
// ---------------------------------------------------------------------------

const DELTA_SPEC: &str = "# Login Specification

## Purpose

Login flow.

### Requirement: Password login

ID: REQ-001
Sources: [legacy]
Status: agreed

The system authenticates users by password.

#### Scenario: Valid credentials

- **WHEN** a user submits valid credentials
- **THEN** the system starts a session
";

/// Seed a `refined` slice tree carrying the fixed artifacts and one
/// domain spec.
fn stage_refined_slice(project: &Project) {
    let slice_dir = project.slice_dir();
    fs::create_dir_all(slice_dir.join("specs/login")).expect("mkdir specs/login");
    fs::write(slice_dir.join("proposal.md"), "# Proposal body\n").expect("write proposal");
    fs::write(slice_dir.join("design.md"), "# Design body\n").expect("write design");
    fs::write(slice_dir.join("tasks.md"), "# Tasks body\n").expect("write tasks");
    fs::write(slice_dir.join("specs/login/spec.md"), DELTA_SPEC).expect("write spec");
    let metadata = SliceMetadata {
        target: "omnia@1.0.0".to_string(),
        status: LifecycleStatus::Refined,
        created_at: Some(now()),
        defined_at: Some(now()),
        completed_at: None,
        merged_at: None,
        dropped_at: None,
        drop_reason: None,
        touched_specs: vec![],
        outcome: None,
    };
    metadata.save(&slice_dir).expect("save metadata");
}

fn success_report() -> BuildReport {
    BuildReport {
        version: 1,
        slice: SLICE_NAME.to_string(),
        target: "omnia@1.0.0".to_string(),
        status: BuildStatus::Success,
        findings: vec![],
        outputs: vec![],
        ui_surface: Some(UiSurface { screens: 0 }),
    }
}

fn tree() -> WorkingTree {
    WorkingTree {
        base: "rev-1".to_string(),
        subpath: None,
    }
}

#[tokio::test]
async fn build_happy_path_runs_finalize_tail() {
    let project = Project::new();
    stage_refined_slice(&project);

    let seam = MockTargetSeam::scripted([], [Ok(success_report())]);
    let outcome = orchestrate::build(&seam, project.layout(), now(), SLICE_NAME, &[], tree())
        .await
        .expect("build succeeds");
    assert_eq!(outcome.slice, SLICE_NAME);
    assert_eq!(outcome.target, "omnia@1.0.0");
    assert_eq!(outcome.status, BuildStatus::Success);
    assert_eq!(outcome.findings, 0);
    assert!(outcome.warnings.is_empty(), "no A4 warnings for a 0-screen slice");

    // Request + report persisted, parity with the native two phases.
    let build_dir = project.slice_dir().join("build");
    let request = fs::read_to_string(build_dir.join("request.yaml")).expect("request persisted");
    assert!(request.contains("specs/login/spec.md"), "{request}");
    let report = fs::read_to_string(build_dir.join("report.yaml")).expect("report persisted");
    assert!(report.contains("status: success"), "{report}");

    // The artifact bodies crossed the seam, routed by the bare target
    // name.
    let calls = seam.calls();
    assert_eq!(calls.len(), 1);
    let TargetCall::Build {
        id,
        slice,
        inputs,
        tree,
    } = &calls[0]
    else {
        panic!("expected a build dispatch, got {calls:?}");
    };
    assert_eq!(id, "target:omnia");
    assert_eq!(slice, SLICE_NAME);
    assert_eq!(tree.base, "rev-1");
    assert_eq!(inputs.len(), 4, "proposal, design, tasks, one spec");
    assert_eq!(inputs[0], specify_workflow::seam::Input::Proposal("# Proposal body\n".into()));
    assert_eq!(inputs[3], specify_workflow::seam::Input::Spec(DELTA_SPEC.into()));

    // The `built` transition landed.
    let metadata = SliceMetadata::load(&project.slice_dir()).expect("reload metadata");
    assert_eq!(metadata.status, LifecycleStatus::Built);

    assert_eq!(
        project.journal_event_ids(),
        ["target.execution.agent", "slice.build.started", "slice.build.succeeded"]
    );
}

fn blocking_finding() -> Diagnostic {
    Diagnostic::finding(
        "OMNIA-001",
        "Provider trait misuse",
        "A provider trait is constructed outside the registry.",
        DiagSeverity::Critical,
        DiagnosticKind::Violation,
        DiagnosticSource::Deterministic,
        Artifact::Code,
        None,
    )
}

#[tokio::test]
async fn build_rejects_blocking_on_success() {
    let project = Project::new();
    stage_refined_slice(&project);

    let report = BuildReport {
        findings: vec![blocking_finding()],
        ..success_report()
    };
    let seam = MockTargetSeam::scripted([], [Ok(report)]);
    let err = orchestrate::build(&seam, project.layout(), now(), SLICE_NAME, &[], tree())
        .await
        .expect_err("blocking finding on success is rejected");
    assert_eq!(err.variant_str(), "target-build-success-with-blocking-finding");

    let metadata = SliceMetadata::load(&project.slice_dir()).expect("reload metadata");
    assert_eq!(metadata.status, LifecycleStatus::Refined, "no transition on rejection");
    assert_eq!(
        project.journal_event_ids(),
        ["target.execution.agent", "slice.build.started", "slice.build.failed"]
    );
    let events = project.journal();
    assert_eq!(events[2]["payload"]["reason"], "target-build-success-with-blocking-finding");
}

#[tokio::test]
async fn build_rejects_missing_outputs() {
    let project = Project::new();
    stage_refined_slice(&project);

    let report = BuildReport {
        outputs: vec![BuildOutput {
            platform: specify_workflow::Platform::Core,
            path: "crates/feature-x/src/lib.rs".to_string(),
        }],
        ..success_report()
    };
    let seam = MockTargetSeam::scripted([], [Ok(report)]);
    let err = orchestrate::build(&seam, project.layout(), now(), SLICE_NAME, &[], tree())
        .await
        .expect_err("absent declared output is rejected");
    assert_eq!(err.variant_str(), "target-build-output-missing");
    let metadata = SliceMetadata::load(&project.slice_dir()).expect("reload metadata");
    assert_eq!(metadata.status, LifecycleStatus::Refined);
}

#[tokio::test]
async fn build_rejects_failure_report() {
    let project = Project::new();
    stage_refined_slice(&project);

    let report = BuildReport {
        status: BuildStatus::Failure,
        findings: vec![blocking_finding()],
        ..success_report()
    };
    let seam = MockTargetSeam::scripted([], [Ok(report)]);
    let err = orchestrate::build(&seam, project.layout(), now(), SLICE_NAME, &[], tree())
        .await
        .expect_err("failure report is rejected");
    assert_eq!(err.variant_str(), "target-build-failed");
    assert_eq!(
        project.journal_event_ids(),
        ["target.execution.agent", "slice.build.started", "slice.build.failed"]
    );
}

#[tokio::test]
async fn build_rejects_slice_mismatch() {
    let project = Project::new();
    stage_refined_slice(&project);

    let report = BuildReport {
        slice: "other-slice".to_string(),
        ..success_report()
    };
    let seam = MockTargetSeam::scripted([], [Ok(report)]);
    let err = orchestrate::build(&seam, project.layout(), now(), SLICE_NAME, &[], tree())
        .await
        .expect_err("mismatched report slice is rejected");
    assert_eq!(err.variant_str(), "target-build-report-slice-mismatch");
}

// ---------------------------------------------------------------------------
// merge
// ---------------------------------------------------------------------------

fn omnia_classes(slice_dir: &Path, project_root: &Path) -> Vec<ArtifactClass> {
    vec![
        ArtifactClass {
            name: "specs".to_string(),
            staged_dir: slice_dir.join("specs"),
            baseline_dir: project_root.join(".specify/specs"),
            strategy: MergeStrategy::ThreeWayMerge,
        },
        ArtifactClass {
            name: "contracts".to_string(),
            staged_dir: slice_dir.join("contracts"),
            baseline_dir: project_root.join("contracts"),
            strategy: MergeStrategy::OpaqueReplace,
        },
    ]
}

const MERGE_PLAN: &str = "\
name: demo
slices:
  - name: feature-x
    status: in-progress
";

/// Seed a `built` slice with one delta spec, ready to merge.
fn stage_built_slice(project: &Project) {
    stage_refined_slice(project);
    let slice_dir = project.slice_dir();
    let mut metadata = SliceMetadata::load(&slice_dir).expect("load metadata");
    metadata.status = LifecycleStatus::Built;
    metadata.save(&slice_dir).expect("save metadata");
}

#[tokio::test]
async fn merge_stamps_done_and_skips_git() {
    let project = Project::new();
    project.seed_plan(MERGE_PLAN);
    stage_built_slice(&project);
    let classes = omnia_classes(&project.slice_dir(), &project.root);

    let outcome = orchestrate::merge(project.layout(), now(), SLICE_NAME, &classes, false)
        .expect("merge succeeds");
    assert_eq!(outcome.merged.len(), 1);
    assert_eq!(outcome.merged[0].name, "login");
    assert_eq!(outcome.archive_path, project.root.join(".specify/archive/2026-01-02-feature-x"));
    assert!(outcome.archive_path.is_dir(), "slice archived");
    assert!(!project.slice_dir().exists(), "slice tree moved to the archive");
    assert!(project.root.join(".specify/specs/login/spec.md").is_file(), "baseline written");

    // The git leg is skipped explicitly and the ledger entry carries no
    // merge-sha.
    assert_eq!(
        project.journal_event_ids(),
        [
            "slice.merge.started",
            "slice.merge.commit-skipped",
            "slice.archive.created",
            "slice.merge.succeeded",
        ]
    );
    let events = project.journal();
    assert_eq!(events[1]["payload"]["slice-name"], SLICE_NAME);
    assert!(
        events[2]["payload"].get("merge-sha").is_none(),
        "guest ledger entry must carry no merge-sha: {}",
        events[2]
    );
    assert_eq!(events[2]["payload"]["touched-specs"][0], "login");

    // `done` stamped on the plan entry.
    let plan = fs::read_to_string(project.root.join("plan.yaml")).expect("read plan");
    assert!(plan.contains("status: done"), "{plan}");
}

#[tokio::test]
async fn merge_rejects_unbuilt_slice() {
    let project = Project::new();
    project.seed_plan(MERGE_PLAN);
    stage_refined_slice(&project);
    let classes = omnia_classes(&project.slice_dir(), &project.root);

    let err = orchestrate::merge(project.layout(), now(), SLICE_NAME, &classes, false)
        .expect_err("a refined slice cannot merge");
    assert_eq!(err.variant_str(), "lifecycle");
    assert_eq!(project.journal_event_ids(), ["slice.merge.started", "slice.merge.failed"]);
    let plan = fs::read_to_string(project.root.join("plan.yaml")).expect("read plan");
    assert!(!plan.contains("status: done"), "no done stamp on failure: {plan}");
}

#[tokio::test]
async fn merge_without_plan_skips_done_stamp() {
    let project = Project::new();
    stage_built_slice(&project);
    let classes = omnia_classes(&project.slice_dir(), &project.root);

    orchestrate::merge(project.layout(), now(), SLICE_NAME, &classes, false)
        .expect("standalone merge succeeds");
    assert!(!project.root.join("plan.yaml").exists());
}

// ---------------------------------------------------------------------------
// synthesize (seam-guidance plumbing)
// ---------------------------------------------------------------------------

const SYNTHESIS_ANSWER: &str = r###"{
  "version": 1,
  "kind": "response",
  "slice": "feature-x",
  "model": {
    "requirements": [{
      "title": "Register with email",
      "statement": "The system accepts registrations with RFC 5322 emails.",
      "domain": "identity",
      "claims": [{ "source": "legacy", "id": "users.register", "kind": "requirement" }]
    }],
    "tasks": [{ "id": "TASK-001", "text": "Implement registration" }]
  },
  "artifacts": {
    "proposal": "## Proposal",
    "design": "## Design",
    "tasks": "## Tasks",
    "specs": [{ "domain": "identity", "content": "## Identity" }]
  }
}"###;

#[tokio::test]
async fn synthesize_reads_seam_guidance() {
    use std::collections::BTreeMap;

    use specify_guest_model::MockModel;
    use specify_model::evidence::{AuthorityClass, ClaimKind};
    use specify_workflow::judgment::synthesize::Kernel;
    use specify_workflow::slice::{BaselineIndex, ProjectionHeader};

    let dir = tempfile::tempdir().expect("tempdir");
    let baseline = BaselineIndex::build(&dir.path().join("specs")).expect("empty baseline");
    let authority = BTreeMap::from([("legacy".to_string(), AuthorityClass::Behaviour)]);
    let claims = BTreeMap::from([(
        ("legacy".to_string(), "users.register".to_string()),
        ClaimKind::Requirement,
    )]);
    let overrides = BTreeMap::new();
    let kernel = Kernel {
        header: ProjectionHeader {
            version: 1,
            slice: SLICE_NAME.to_string(),
            project: None,
        },
        authority: &authority,
        overrides: &overrides,
        evidence_claims: &claims,
        baseline_index: &baseline,
    };

    let model = MockModel::answering([SYNTHESIS_ANSWER]);
    let seam = MockTargetSeam::scripted([Ok("Guidance body from the seam.".to_string())], []);
    let request = orchestrate::SynthesizeRequest {
        slice: SLICE_NAME,
        target: "omnia",
        sources: &[],
        baseline: &[],
        baseline_detail: &[],
    };

    let synthesized = orchestrate::synthesize(&model, &seam, &request, &kernel)
        .await
        .expect("synthesis succeeds");
    assert_eq!(synthesized.response.slice, SLICE_NAME);

    // Guidance was routed by the target adapter id and rode the inputs
    // envelope in place of the manifest shape brief.
    assert_eq!(
        seam.calls(),
        [TargetCall::Guidance {
            id: "target:omnia".to_string()
        }]
    );
    let calls = model.requests();
    assert_eq!(calls.len(), 1);
    assert!(
        calls[0].messages[0].content.contains("Guidance body from the seam."),
        "the seam guidance rides the synthesis inputs"
    );
}

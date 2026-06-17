//! Integration tests for `specify source survey`
//! (DECISIONS.md §"Source operations").
//!
//! Covers source resolution against `plan.yaml.sources`, the agent
//! two-phase dispatch (prepare prints the handoff envelope + emits
//! `source.execution.agent`; finalize validates-before-visible and
//! emits `source.survey.completed`), and the validate-before-visible
//! guarantee that an invalid lead set leaves `discovery.md` untouched.

use std::fs;
use std::path::PathBuf;

use crate::common::{
    Project, parse_stderr, parse_stdout, read_journal_normalized, repo_root, specify_cmd,
};

fn stage_typescript(project: &Project) {
    // The in-repo fixture ships only `adapter.yaml` (execution: agent);
    // stage it, then author the `survey` brief the agent reads.
    let src = repo_root()
        .join("crates/workflow/tests/fixtures/plugins/adapters/sources/typescript/adapter.yaml");
    let adapter_dir = project.root().join("adapters/sources/typescript");
    fs::create_dir_all(adapter_dir.join("briefs")).expect("create adapter briefs dir");
    fs::copy(&src, adapter_dir.join("adapter.yaml")).expect("copy adapter.yaml");
    fs::write(adapter_dir.join("briefs/survey.md"), "# survey brief\n")
        .expect("write survey brief");
}

fn seed_plan_with_legacy_source(project: &Project) {
    project.seed_plan(
        "name: platform-v2
sources:
  legacy:
    adapter: typescript
    path: vendor/legacy
slices:
  - name: a
    project: default
    status: pending
",
    );
}

fn survey_scratch_dir(project: &Project) -> PathBuf {
    project.root().join(".specify/scratch/typescript/survey")
}

// A `survey` lead-set omits `source`: attribution is CLI-owned,
// so the runner stamps `legacy` onto every lead before the schema
// check and the merge.
const VALID_LEAD_SET: &str = "\
### user-registration

- lead: user-registration
- synopsis: Registration endpoint accepting email + password.
";

#[test]
fn prepare_prints_envelope_emits_event() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "survey", "legacy"])
        .assert()
        .success();

    let body = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(body["adapter"], "typescript");
    assert_eq!(body["version"], "1.0.0");
    assert_eq!(body["execution"], "agent");
    assert!(
        body.get("evidence-dir").is_none(),
        "survey handoff must NOT carry evidence-dir, got:\n{body}"
    );
    let scratch = body["scratch-dir"].as_str().expect("scratch-dir str");
    assert!(
        scratch.ends_with(".specify/scratch/typescript/survey"),
        "scratch-dir {scratch} must key under the survey segment"
    );
    let briefs = body["briefs-dir"].as_str().expect("briefs-dir str");
    assert!(briefs.ends_with("adapters/sources/typescript/briefs"), "briefs-dir: {briefs}");
    let source_dir = body["source-dir"].as_str().expect("source-dir str");
    assert!(source_dir.ends_with("vendor/legacy"), "source-dir: {source_dir}");
    assert_eq!(
        body["leads"].as_array().expect("leads array").len(),
        0,
        "fresh survey has no leads"
    );

    // prepare builds the scratch dir up front.
    assert!(survey_scratch_dir(&project).is_dir(), "prepare must create the scratch dir");

    let events = read_journal_normalized(project.root());
    assert_eq!(events.len(), 1, "prepare emits exactly one event");
    assert_eq!(events[0]["event"], "source.execution.agent");
    assert_eq!(events[0]["payload"]["source"], "legacy");
    assert_eq!(events[0]["payload"]["adapter"], "typescript");
    assert_eq!(events[0]["payload"]["operation"], "survey");
}

#[test]
fn prepare_clears_stale_scratch() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    // A prior run left a stale lead set in the scratch lane.
    let scratch = survey_scratch_dir(&project);
    fs::create_dir_all(&scratch).expect("create scratch dir");
    fs::write(scratch.join("leads.md"), "### stale-lead\n").expect("write stale leads.md");

    specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "survey", "legacy"])
        .assert()
        .success();

    assert!(scratch.is_dir(), "prepare must recreate the scratch dir");
    assert!(
        !scratch.join("leads.md").exists(),
        "prepare must drop stale artifacts so finalize only sees this run's output"
    );
}

#[test]
fn finalize_merges_and_completes() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    // Stand in for the agent: write the produced lead set into scratch.
    let scratch = survey_scratch_dir(&project);
    fs::create_dir_all(&scratch).expect("create scratch dir");
    fs::write(scratch.join("leads.md"), VALID_LEAD_SET).expect("write leads.md");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "survey", "legacy", "--phase", "finalize"])
        .assert()
        .success();

    let body = parse_stdout(&assert.get_output().stdout, project.root());
    assert_eq!(body["adapter"], "typescript");
    assert_eq!(body["source"], "legacy");
    let leads: Vec<&str> =
        body["leads"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();
    assert_eq!(leads, vec!["user-registration"]);

    // The lead is now visible in discovery.md.
    let discovery = fs::read_to_string(project.root().join("discovery.md")).expect("discovery.md");
    assert!(
        discovery.contains("### legacy:user-registration"),
        "merged lead must appear:\n{discovery}"
    );
    assert!(discovery.contains("- source: legacy"), "merged lead records its source");

    let events = read_journal_normalized(project.root());
    let completed = events
        .iter()
        .find(|e| e["event"] == "source.survey.completed")
        .expect("a survey-completed event");
    assert_eq!(completed["payload"]["source"], "legacy");
    assert_eq!(completed["payload"]["adapter"], "typescript");
}

#[test]
fn finalize_unparseable_lead_set_errors() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    let scratch = survey_scratch_dir(&project);
    fs::create_dir_all(&scratch).expect("create scratch dir");
    fs::write(scratch.join("leads.md"), "The survey found registration behavior.\n")
        .expect("write unparseable leads.md");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "survey", "legacy", "--phase", "finalize"])
        .assert()
        .failure();

    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "survey-leads-empty");
    assert_eq!(stderr["exit-code"], 1);
    assert!(
        !project.root().join("discovery.md").exists(),
        "an unparseable lead set must leave discovery.md untouched"
    );
    assert!(
        !project.root().join(".specify/journal.jsonl").exists()
            || !read_journal_normalized(project.root())
                .iter()
                .any(|e| e["event"] == "source.survey.completed"),
        "unparseable lead set must not emit a completion event"
    );
}

#[test]
fn finalize_invalid_lead_set_untouched() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    // `bad_id` parses as a lead block but fails the kebab-case schema.
    let scratch = survey_scratch_dir(&project);
    fs::create_dir_all(&scratch).expect("create scratch dir");
    fs::write(
        scratch.join("leads.md"),
        "## Lead inventory\n\n### bad_id\n\n- lead: bad_id\n- synopsis: Bad id.\n",
    )
    .expect("write invalid leads.md");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "survey", "legacy", "--phase", "finalize"])
        .assert()
        .failure();

    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "discovery-lead-schema");
    assert_eq!(stderr["exit-code"], 2);

    // Validate-before-visible: nothing was written.
    assert!(
        !project.root().join("discovery.md").exists(),
        "an invalid lead set must leave discovery.md untouched"
    );
    // No completion event fires for an invalid lead set.
    assert!(
        !project.root().join(".specify/journal.jsonl").exists()
            || !read_journal_normalized(project.root())
                .iter()
                .any(|e| e["event"] == "source.survey.completed"),
        "invalid lead set must not emit a completion event"
    );
}

#[test]
fn unknown_source_errors() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "survey", "not-a-source"])
        .assert()
        .failure();

    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "source-unknown");
    assert_eq!(stderr["exit-code"], 1);
}

#[test]
fn plan_name_mismatch_errors() {
    let project = Project::init();
    stage_typescript(&project);
    seed_plan_with_legacy_source(&project);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "source", "survey", "legacy", "--plan", "wrong-plan"])
        .assert()
        .failure();

    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["exit-code"], 2, "a --plan mismatch is an argument error");
}

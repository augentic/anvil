//! Wire-contract coverage for `system survey`: the definition home
//! is the anchored root (no `.emery/`, no `project.yaml`), declared
//! inputs fail closed, included rows survey → extract with coverage
//! accounting, the gates stop typed, and correlation persists `as-is`.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use native::{DynModel, Provider, ReferenceMode};
use omnia_guest::api::invoke::Invoker;
use omnia_testkit::model::Harness;

fn provider(root: impl Into<PathBuf>, answers: Vec<String>) -> Provider {
    let root = root.into();
    let locations = project::handler::Locations::explicit(
        root.join("store"),
        project::handler::CachePlacement::Parent(root.join("project-cache")),
    );
    Provider::new(
        project::handler::ExecutionPaths::new(root, locations),
        DynModel::new(Harness::answering(answers)),
        mock::catalog(),
        ReferenceMode::Offline,
    )
}

/// A minimal valid correlation answer: one inferred element.
const CORRELATED: &str = r#"{"version":1,"kind":"response","as-is":{"elements":[{"id":"orders","kind":"service","status":"inferred","claims":[]}],"relationships":[]}}"#;

const SCOPE: &str = "version: 1\nid: acme-estate\ndecision: consolidate the order stack\n";

/// One included `./orders` row bound to `adapter`, plus an
/// operator-accounted inaccessible row.
fn coverage(adapter: &str) -> String {
    format!(
        "version: 1\ncandidates:\n  - key: orders-code\n    location: ./orders\n    adapter: \
         {adapter}\n    disposition: included\n    reason: primary implementation\n  - key: \
         legacy-erp\n    location: https://erp.example.com\n    disposition: inaccessible\n    \
         reason: vendor system, no export\n"
    )
}

/// Author a definition home: scope, coverage, and the `./orders`
/// tree the included row points at.
fn author_home(home: &Path, coverage_yaml: &str) {
    fs::write(home.join("scope.yaml"), SCOPE).expect("scope.yaml");
    fs::write(home.join("coverage.yaml"), coverage_yaml).expect("coverage.yaml");
    fs::create_dir_all(home.join("orders")).expect("orders tree");
    fs::write(home.join("orders/main.ts"), "export {};\n").expect("orders file");
}

/// Dispatch one `system *` argv in JSON format against `home`, with
/// `answers` scripted onto the judgment model.
async fn dispatch_json(home: &Path, answers: Vec<String>, argv: &[&str]) -> (u8, String) {
    let router =
        transport::command::router(Invoker::new("emery", provider(home, answers))).expect("router");
    let mut full = vec!["emery", "--format", "json"];
    full.extend_from_slice(argv);
    let response = router.execute(full).await;
    let stream = if response.exit == 0 { response.stdout } else { response.stderr };
    (response.exit, String::from_utf8(stream).expect("output is UTF-8"))
}

/// Dispatch `system survey` argv in JSON format against `home`, with
/// `answers` scripted onto the correlation model.
async fn survey_json(home: &Path, answers: Vec<String>) -> (u8, String) {
    dispatch_json(home, answers, &["system", "survey"]).await
}

#[tokio::test]
async fn missing_scope_fails_typed() {
    let home = tempfile::tempdir().expect("tempdir");
    let (exit, stderr) = survey_json(home.path(), Vec::new()).await;
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("error envelope");
    assert_ne!(exit, 0);
    assert_eq!(envelope["error"], "system-scope-missing");
    let hint = envelope["hint"].as_str().expect("hint carries the two-file template");
    assert!(hint.contains("scope.yaml:"), "hint prints the scope template: {hint}");
    assert!(hint.contains("coverage.yaml:"), "hint prints the coverage template: {hint}");
}

#[tokio::test]
async fn coverage_missing_typed() {
    let home = tempfile::tempdir().expect("tempdir");
    fs::write(home.path().join("scope.yaml"), SCOPE).expect("scope.yaml");
    let (exit, stderr) = survey_json(home.path(), Vec::new()).await;
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("error envelope");
    assert_ne!(exit, 0);
    assert_eq!(envelope["error"], "system-coverage-missing");
}

#[tokio::test]
async fn surveyed_home() {
    let home = tempfile::tempdir().expect("tempdir");
    author_home(home.path(), &coverage("mock"));

    let (exit, stdout) = survey_json(home.path(), vec![CORRELATED.to_string()]).await;
    assert_eq!(exit, 0, "declared home surveys: {stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    assert_eq!(body["id"], "acme-estate");
    assert_eq!(body["candidates"], 2);
    assert_eq!(body["evidence"], 1);
    let source = &body["sources"][0];
    assert_eq!(source["status"], "surveyed");
    assert_eq!(source["source"], "orders-code");
    assert_eq!(source["adapter"], "mock");
    assert_eq!(source["leads"], 1);
    let cid = source["observed-cid"].as_str().expect("observed cid on the body");
    assert!(cid.starts_with("sha256:"), "{cid}");

    // Evidence persisted by `(source, lead)` under the home.
    let evidence = home.path().join("evidence/orders-code/greeting.yaml");
    let document = fs::read_to_string(evidence).expect("persisted Evidence");
    assert!(document.contains("authority: documentation"), "{document}");

    // The coverage row gained the observed tree surgically: declared
    // fields intact, the accounting row untouched.
    let coverage = fs::read_to_string(home.path().join("coverage.yaml")).expect("coverage.yaml");
    assert!(coverage.contains(&format!("observed-cid: {cid}")), "{coverage}");
    assert!(coverage.contains("legacy-erp"), "{coverage}");
    assert!(coverage.contains("vendor system, no export"), "{coverage}");

    // Generated layout grew beneath the home, and correlation wrote
    // the `as-is` state through the overlay persist.
    assert!(home.path().join("events").is_dir(), "events/ is generated on first success");
    assert_eq!(body["as-is"]["elements"], 1, "{body}");
    let system = fs::read_to_string(home.path().join("system.yaml")).expect("system.yaml");
    assert!(system.contains("identities: []"), "first creation mints empty identities: {system}");
    assert!(system.contains("id: orders"), "{system}");
    let view = fs::read_to_string(home.path().join("architecture/as-is.md")).expect("projection");
    assert!(view.contains("Digest: sha256:"), "the view is digest-stamped: {view}");
}

#[tokio::test]
async fn failures_accounted() {
    // An access failure (missing tree) and an adapter failure (survey
    // refuses) both stay durable coverage rows with `survey-error`.
    let home = tempfile::tempdir().expect("tempdir");
    let coverage_yaml = "version: 1\ncandidates:\n  - key: broken-src\n    location: ./missing\n    \
                         adapter: mock\n    disposition: included\n    reason: gone\n  - key: \
                         flaky\n    location: ./orders\n    adapter: mock-fail-survey\n    \
                         disposition: included\n    reason: flaky adapter\n";
    author_home(home.path(), coverage_yaml);

    let (exit, stdout) = survey_json(home.path(), Vec::new()).await;
    assert_eq!(exit, 0, "failed rows are accounting, not a run failure: {stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    assert_eq!(body["evidence"], 0);
    let kinds: Vec<(&str, &str)> = body["sources"]
        .as_array()
        .expect("sources")
        .iter()
        .map(|source| {
            (source["source"].as_str().expect("key"), source["kind"].as_str().expect("kind"))
        })
        .collect();
    assert!(kinds.contains(&("broken-src", "access")), "{kinds:?}");
    assert!(kinds.contains(&("flaky", "adapter")), "{kinds:?}");

    let coverage = fs::read_to_string(home.path().join("coverage.yaml")).expect("coverage.yaml");
    assert!(coverage.contains("survey-error"), "{coverage}");
    assert!(!coverage.contains("observed-cid"), "no row completed: {coverage}");

    // Zero included Evidence still completes: empty `as-is` persists
    // deterministically without a model call.
    assert_eq!(body["as-is"]["elements"], 0, "{body}");
    let system = fs::read_to_string(home.path().join("system.yaml")).expect("system.yaml");
    assert!(system.contains("as-is"), "{system}");
}

#[tokio::test]
async fn extract_keeps_prior() {
    // A failed extract records `survey-error` and neither replaces the
    // prior Evidence corpus nor updates the observed tree.
    let home = tempfile::tempdir().expect("tempdir");
    author_home(home.path(), &coverage("mock-fail-extract"));
    let prior = home.path().join("evidence/orders-code/prior.yaml");
    fs::create_dir_all(prior.parent().expect("parent")).expect("prior dir");
    fs::write(&prior, "lead: prior\nauthority: documentation\nclaims: []\n").expect("prior doc");

    // The prior corpus is still the correlation input, so the run
    // makes one model call over it.
    let (exit, stdout) = survey_json(home.path(), vec![CORRELATED.to_string()]).await;
    assert_eq!(exit, 0, "{stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    assert_eq!(body["sources"][0]["status"], "failed");
    assert_eq!(body["sources"][0]["kind"], "adapter");
    assert!(prior.exists(), "a failed source keeps its prior Evidence corpus");

    let coverage = fs::read_to_string(home.path().join("coverage.yaml")).expect("coverage.yaml");
    assert!(!coverage.contains("observed-cid"), "{coverage}");
}

#[tokio::test]
async fn lead_gate_stops() {
    // 86 included `mock-docs` rows survey 258 leads — over the engine
    // ceiling. The stop is typed and nothing extracts, but surveyed
    // rows keep their observed trees (the gate is extract-side).
    let home = tempfile::tempdir().expect("tempdir");
    let mut coverage_yaml = "version: 1\ncandidates:\n".to_string();
    for row in 0..86 {
        write!(
            coverage_yaml,
            "  - key: src-{row}\n    location: ./orders\n    adapter: mock-docs\n    disposition: \
             included\n    reason: fan-out\n"
        )
        .expect("string write");
    }
    author_home(home.path(), &coverage_yaml);

    let (exit, stderr) = survey_json(home.path(), Vec::new()).await;
    assert_ne!(exit, 0);
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("error envelope");
    assert_eq!(envelope["error"], "system-survey-lead-limit");
    assert!(!home.path().join("evidence").exists(), "a gate stop never extracts");
    assert!(!home.path().join("system.yaml").exists(), "a gate stop writes no as-is");
    let coverage = fs::read_to_string(home.path().join("coverage.yaml")).expect("coverage.yaml");
    assert!(coverage.contains("observed-cid"), "surveyed rows keep provenance: {coverage}");
}

#[tokio::test]
async fn claim_gate_stops() {
    // A prior corpus over the claim ceiling stops typed before the
    // judgment and does not write (or replace) `as-is`.
    let home = tempfile::tempdir().expect("tempdir");
    author_home(home.path(), &coverage("mock-fail-survey"));
    let mut doc = "lead: prior\nauthority: documentation\nclaims:\n".to_string();
    for claim in 0..4097 {
        write!(doc, "  - kind: requirement\n    id: bulk.c{claim}\n").expect("string write");
    }
    let prior = home.path().join("evidence/orders-code/prior.yaml");
    fs::create_dir_all(prior.parent().expect("parent")).expect("prior dir");
    fs::write(&prior, doc).expect("prior doc");

    let (exit, stderr) = survey_json(home.path(), Vec::new()).await;
    assert_ne!(exit, 0);
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("error envelope");
    assert_eq!(envelope["error"], "system-correlation-claim-limit");
    assert!(!home.path().join("system.yaml").exists(), "a gate stop writes no as-is");
}

/// A minimal valid initial-plan proposal: one-element target, one
/// disposition, one wave selecting the surveyed `greeting` lead.
const PROPOSED: &str = r#"{"version":1,"kind":"response","target":{"elements":[{"id":"orders","kind":"service","status":"inferred","claims":[]}],"relationships":[]},"dispositions":[{"id":"keep-orders","treatment":"preserve","applies-to":["orders"],"reason":"orders behaviour survives"}],"wave":{"id":"wave-1","outcome":"replatform orders","architecture":{"before":"as-is","after":"target"},"affected-elements":["orders"],"evidence-scopes":[{"source":"orders-code","lead":"greeting"}]}}"#;

#[tokio::test]
async fn plan_requires_survey() {
    // Plan projects from the surveyed model; a home that never
    // surveyed fails typed before any judgment.
    let home = tempfile::tempdir().expect("tempdir");
    author_home(home.path(), &coverage("mock"));
    let (exit, stderr) = dispatch_json(home.path(), Vec::new(), &["system", "plan"]).await;
    assert_ne!(exit, 0);
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("error envelope");
    assert_eq!(envelope["error"], "system-model-missing");
    assert!(
        envelope["message"].as_str().expect("message").contains("emery system survey"),
        "{envelope}"
    );
}

#[tokio::test]
async fn plan_review_loop() {
    let home = tempfile::tempdir().expect("tempdir");
    author_home(home.path(), &coverage("mock"));
    let (exit, _stdout) = survey_json(home.path(), vec![CORRELATED.to_string()]).await;
    assert_eq!(exit, 0);

    // First plan: the proposal judgment writes `target` and mints
    // migration.yaml; every view and the wave handoff project.
    let (exit, stdout) =
        dispatch_json(home.path(), vec![PROPOSED.to_string()], &["system", "plan"]).await;
    assert_eq!(exit, 0, "plan proposes: {stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    assert_eq!(body["proposed"], true);
    let states: Vec<&str> = body["states"]
        .as_array()
        .expect("states")
        .iter()
        .map(|state| state.as_str().expect("name"))
        .collect();
    assert_eq!(states, ["as-is", "target"]);
    assert_eq!(body["waves"][0]["wave"], "wave-1");
    let digest = body["waves"][0]["handoff-digest"].as_str().expect("digest on the body");
    let stem = digest.strip_prefix("sha256:").expect("scheme-prefixed digest");

    let system = fs::read_to_string(home.path().join("system.yaml")).expect("system.yaml");
    assert!(system.contains("target:"), "the proposal persisted: {system}");
    let migration = fs::read_to_string(home.path().join("migration.yaml")).expect("migration");
    assert!(migration.contains("keep-orders"), "{migration}");
    assert!(home.path().join("architecture/target.md").is_file(), "target view projected");
    assert!(home.path().join("architecture/diagrams/target.svg").is_file());
    assert!(home.path().join(format!("handoffs/{stem}.yaml")).is_file());

    // Re-running plan is resume: `target` is present, so no judgment
    // runs (no scripted answers), nothing is overwritten, and the
    // handoff digest is reproduced deterministically.
    let (exit, stdout) = dispatch_json(home.path(), Vec::new(), &["system", "plan"]).await;
    assert_eq!(exit, 0, "re-run is resume: {stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    assert_eq!(body["proposed"], false);
    assert_eq!(body["waves"][0]["handoff-digest"], digest);

    // Status projects the awaiting-review wave as the next action.
    let (exit, stdout) = dispatch_json(home.path(), Vec::new(), &["system", "status"]).await;
    assert_eq!(exit, 0, "{stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    assert_eq!(body["waves"][0]["standing"], "awaiting-review");
    assert!(body["next"].as_str().expect("next").contains("review wave-1"), "{body}");

    // A stale digest is refused before any fact is written.
    let (exit, stderr) = dispatch_json(
        home.path(),
        Vec::new(),
        &["system", "review", "wave-1", "--handoff", &"0".repeat(64)],
    )
    .await;
    assert_ne!(exit, 0);
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("error envelope");
    assert_eq!(envelope["error"], "system-review-stale");

    // Reviewing the current handoff appends the fact once.
    let (exit, stdout) = dispatch_json(
        home.path(),
        Vec::new(),
        &["system", "review", "wave-1", "--handoff", digest],
    )
    .await;
    assert_eq!(exit, 0, "{stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    assert_eq!(body["recorded"], true);
    let log = fs::read_to_string(home.path().join("events/local.jsonl")).expect("fact log");
    assert!(log.contains("system.wave.reviewed"), "{log}");
    assert!(log.contains(digest), "the fact pins the exact handoff: {log}");

    // Same-handoff re-entry is a read-only no-op.
    let (exit, stdout) = dispatch_json(
        home.path(),
        Vec::new(),
        &["system", "review", "wave-1", "--handoff", digest],
    )
    .await;
    assert_eq!(exit, 0, "{stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    assert_eq!(body["recorded"], false);
    let log = fs::read_to_string(home.path().join("events/local.jsonl")).expect("fact log");
    assert_eq!(log.lines().count(), 1, "no duplicate fact: {log}");

    // Status now projects the reviewed loop end.
    let (exit, stdout) = dispatch_json(home.path(), Vec::new(), &["system", "status"]).await;
    assert_eq!(exit, 0, "{stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    assert_eq!(body["waves"][0]["standing"], "reviewed");
    assert_eq!(body["next"], "reviewed");
}

#[tokio::test]
async fn plan_over_empty_as_is() {
    // AC11: a definition whose survey produced no Evidence (empty
    // `as-is`) still plans — the proposal writes `target` plus one
    // evidence-collection wave with no targets and no mappings.
    let home = tempfile::tempdir().expect("tempdir");
    author_home(home.path(), &coverage("mock-fail-survey"));
    let (exit, _stdout) = survey_json(home.path(), Vec::new()).await;
    assert_eq!(exit, 0, "a failed source still persists empty as-is");

    let proposal = r#"{"version":1,"kind":"response","target":{"elements":[{"id":"orders","kind":"service","status":"inferred","claims":[]}],"relationships":[]},"wave":{"id":"wave-0","outcome":"collect the missing evidence","architecture":{"before":"as-is","after":"as-is"},"gaps":[{"id":"g1","detail":"orders source inaccessible this run"}]}}"#;
    let (exit, stdout) =
        dispatch_json(home.path(), vec![proposal.to_string()], &["system", "plan"]).await;
    assert_eq!(exit, 0, "empty as-is still plans: {stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    assert_eq!(body["proposed"], true);
    assert_eq!(body["waves"][0]["wave"], "wave-0");
    let migration = fs::read_to_string(home.path().join("migration.yaml")).expect("migration");
    assert!(migration.contains("collect the missing evidence"), "{migration}");
}

#[tokio::test]
async fn operator_plan_edit_moves_handoff() {
    // An operator edit to migration.yaml is never overwritten: the
    // next plan reprojects a new handoff beside the historical one,
    // and a review of the old digest is refused.
    let home = tempfile::tempdir().expect("tempdir");
    author_home(home.path(), &coverage("mock"));
    let (exit, _stdout) = survey_json(home.path(), vec![CORRELATED.to_string()]).await;
    assert_eq!(exit, 0);
    let (exit, stdout) =
        dispatch_json(home.path(), vec![PROPOSED.to_string()], &["system", "plan"]).await;
    assert_eq!(exit, 0, "{stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    let first = body["waves"][0]["handoff-digest"].as_str().expect("digest").to_string();

    let path = home.path().join("migration.yaml");
    let edited = fs::read_to_string(&path)
        .expect("migration.yaml")
        .replace("replatform orders", "rewrite orders in place");
    fs::write(&path, &edited).expect("operator edit");

    let (exit, stdout) = dispatch_json(home.path(), Vec::new(), &["system", "plan"]).await;
    assert_eq!(exit, 0, "{stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    assert_eq!(body["proposed"], false);
    let second = body["waves"][0]["handoff-digest"].as_str().expect("digest").to_string();
    assert_ne!(second, first, "the edited wave projects a new handoff");
    let migration = fs::read_to_string(&path).expect("migration.yaml");
    assert!(migration.contains("rewrite orders in place"), "operator edit survives: {migration}");

    // Both content-addressed files remain; only the current reviews.
    let handoffs = fs::read_dir(home.path().join("handoffs")).expect("handoffs").count();
    assert_eq!(handoffs, 2, "historical handoffs are never deleted");
    let (exit, stderr) = dispatch_json(
        home.path(),
        Vec::new(),
        &["system", "review", "wave-1", "--handoff", &first],
    )
    .await;
    assert_ne!(exit, 0);
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("error envelope");
    assert_eq!(envelope["error"], "system-review-stale");
}

#[tokio::test]
async fn claims_close_in_loop() {
    // An answer citing a claim no Evidence document carries fails the
    // deterministic tail and is repaired in-loop; the corrected answer
    // persists.
    let home = tempfile::tempdir().expect("tempdir");
    author_home(home.path(), &coverage("mock"));
    let bogus = r#"{"version":1,"kind":"response","as-is":{"elements":[{"id":"orders","kind":"service","status":"evidenced","claims":[{"source":"orders-code","id":"no.such-claim"}]}],"relationships":[]}}"#;

    let (exit, stdout) =
        survey_json(home.path(), vec![bogus.to_string(), CORRELATED.to_string()]).await;
    assert_eq!(exit, 0, "the repair loop corrects the citation: {stdout}");
    let body: serde_json::Value = serde_json::from_str(&stdout).expect("success envelope");
    assert_eq!(body["as-is"]["elements"], 1, "{body}");
    let system = fs::read_to_string(home.path().join("system.yaml")).expect("system.yaml");
    assert!(system.contains("status: inferred"), "the repaired answer persisted: {system}");
}

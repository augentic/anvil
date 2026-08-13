//! Wire-contract coverage for `system survey`: the definition home
//! is the anchored root (no `.emery/`, no `project.yaml`), declared
//! inputs fail closed, included rows survey → extract with coverage
//! accounting, and the lead gate stops before any extract.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use native::{DynModel, Provider, ReferenceMode};
use omnia_guest::api::invoke::Invoker;
use omnia_testkit::model::Harness;

fn provider(root: impl Into<PathBuf>) -> Provider {
    let root = root.into();
    let locations = project::handler::Locations::explicit(
        root.join("store"),
        project::handler::CachePlacement::Parent(root.join("project-cache")),
    );
    Provider::new(
        project::handler::ExecutionPaths::new(root, locations),
        DynModel::new(Harness::answering(Vec::<String>::new())),
        mock::catalog(),
        ReferenceMode::Offline,
    )
}

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

/// Dispatch `system survey` argv in JSON format against `home`.
async fn survey_json(home: &Path) -> (u8, String) {
    let router = transport::command::router(Invoker::new("emery", provider(home))).expect("router");
    let response = router.execute(["emery", "--format", "json", "system", "survey"]).await;
    let stream = if response.exit == 0 { response.stdout } else { response.stderr };
    (response.exit, String::from_utf8(stream).expect("output is UTF-8"))
}

#[tokio::test]
async fn missing_scope_fails_typed() {
    let home = tempfile::tempdir().expect("tempdir");
    let (exit, stderr) = survey_json(home.path()).await;
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
    let (exit, stderr) = survey_json(home.path()).await;
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("error envelope");
    assert_ne!(exit, 0);
    assert_eq!(envelope["error"], "system-coverage-missing");
}

#[tokio::test]
async fn surveyed_home() {
    let home = tempfile::tempdir().expect("tempdir");
    author_home(home.path(), &coverage("mock"));

    let (exit, stdout) = survey_json(home.path()).await;
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

    // Generated layout grew beneath the home.
    assert!(home.path().join("events").is_dir(), "events/ is generated on first success");
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

    let (exit, stdout) = survey_json(home.path()).await;
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

    let (exit, stdout) = survey_json(home.path()).await;
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

    let (exit, stderr) = survey_json(home.path()).await;
    assert_ne!(exit, 0);
    let envelope: serde_json::Value = serde_json::from_str(&stderr).expect("error envelope");
    assert_eq!(envelope["error"], "system-survey-lead-limit");
    assert!(!home.path().join("evidence").exists(), "a gate stop never extracts");
    let coverage = fs::read_to_string(home.path().join("coverage.yaml")).expect("coverage.yaml");
    assert!(coverage.contains("observed-cid"), "surveyed rows keep provenance: {coverage}");
}

//! Integration tests for `specify catalog infer`.
//!
//! These tests assert the host **mechanism** — dispatch wiring, the report
//! shape, and the bind guards (stability / uniqueness / no-overwrite) — and
//! never that a specific English name like `tab-bar` emerges, because
//! naming is the build skill's judgement, not the CLI's. Where
//! a bound name is needed, the test supplies a fixed `{ fingerprint →
//! slug }` bindings map standing in for the agent's decision.
//!
//! Tests that dispatch the `dispatch-fixture` WASI component use the
//! checked-in `adapter.wasm`; bind-only guard tests need no WASM.

use std::fs;
use std::path::{Path, PathBuf};

use common::{
    dispatch_fixture_adapter_dir, parse_json, parse_stderr, specify_cmd, stage_dispatch_fixture,
};
use serde_json::Value;
use specify_workflow::design_system::{ComponentStatus, ComponentsCatalog};
use tempfile::{TempDir, tempdir};

use crate::common;

/// Default cluster fingerprint emitted by `dispatch-fixture infer`.
const FIXTURE_DEFAULT_FP: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
/// Pinned-cluster fingerprint when `dispatch-fixture infer` receives `--parts`.
const FIXTURE_PINNED_FP: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
/// Unpinned cluster fingerprint for slug-collision scenarios with `--parts`.
const FIXTURE_UNPINNED_FP: &str =
    "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";

/// A baseline with the same `footer` group on three screens (clusters
/// to one entry at the default threshold of 2) plus a unique `body`
/// group on a single screen (below threshold, absent from the report).
const REPEATED_GROUP_BASELINE: &str = "version: 1
screens:
  home:
    name: Home
    footer:
      - group:
          items:
            - icon-button: { bind: home, event: Navigate(Home) }
            - icon-button: { bind: search, event: Navigate(Search) }
  search:
    name: Search
    footer:
      - group:
          items:
            - icon-button: { bind: home, event: Navigate(Home) }
            - icon-button: { bind: search, event: Navigate(Search) }
    body:
      - group:
          items:
            - text: {}
";

/// Scaffold a minimal `.specify/` project with a `project.yaml` the
/// `bind`-phase handler can load. `bind` resolves no adapter and runs no
/// tool, so this is all the bind tests need.
fn bind_project() -> TempDir {
    let tmp = tempdir().expect("tempdir");
    fs::create_dir_all(tmp.path().join(".specify")).expect("create .specify");
    fs::write(
        tmp.path().join(".specify/project.yaml"),
        "name: catalog-test\nadapter: dispatch-fixture\nrules: {}\n",
    )
    .expect("write project.yaml");
    tmp
}

/// Scaffold a project with the `dispatch-fixture` adapter staged and a
/// composition baseline — everything the `report` phase needs to dispatch.
/// Returns the project tempdir and the extensions cache dir.
fn report_project(baseline: &str) -> (TempDir, PathBuf) {
    let tmp = tempdir().expect("tempdir");
    let project = tmp.path();
    stage_dispatch_fixture(project);
    fs::create_dir_all(project.join(".specify/specs")).expect("create specs");
    fs::write(
        project.join(".specify/project.yaml"),
        "name: catalog-test\nadapter: dispatch-fixture\nrules: {}\n",
    )
    .expect("write project.yaml");
    fs::write(project.join(".specify/specs/composition.yaml"), baseline)
        .expect("write composition.yaml");

    let cache = tmp.path().join("tools-cache");
    fs::create_dir_all(&cache).expect("create cache");
    (tmp, cache)
}

/// Write a bindings file under `project/.specify/` and return its path.
fn write_bindings(project: &Path, body: &str) -> PathBuf {
    let path = project.join(".specify/bindings.yaml");
    fs::write(&path, body).expect("write bindings");
    path
}

/// Write an operator `parts.yaml` under `project/.specify/design-system/`.
fn write_parts(project: &Path, body: &str) {
    let dir = project.join(".specify/design-system");
    fs::create_dir_all(&dir).expect("create design-system dir");
    fs::write(dir.join("parts.yaml"), body).expect("write parts.yaml");
}

/// A baseline with a single `footer` group on one screen (below the
/// default threshold of 2 on its own).
const SINGLE_FOOTER_BASELINE: &str = "version: 1
screens:
  home:
    name: Home
    footer:
      - group:
          items:
            - icon-button: { bind: home, event: Navigate(Home) }
            - icon-button: { bind: search, event: Navigate(Search) }
";

/// An operator part whose `group` skeleton (two icon-buttons) matches
/// the `SINGLE_FOOTER_BASELINE` footer group.
const PRIMARY_NAV_PART: &str = "version: 1
parts:
  primary-nav:
    description: Operator-defined nav bar.
    group:
      items:
        - icon-button: { bind: a, event: Navigate(A) }
        - icon-button: { bind: b, event: Navigate(B) }
";

fn load_catalog(project: &Path) -> Option<ComponentsCatalog> {
    ComponentsCatalog::load(project).expect("catalog loads")
}

#[test]
fn report_dispatches_fixture_clusters() {
    let (tmp, cache) = report_project(REPEATED_GROUP_BASELINE);
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["--format", "json", "catalog", "infer", "--phase", "report"])
        .assert()
        .success();

    let report = parse_json(&assert.get_output().stdout);
    assert_eq!(report["version"], 1);
    let clusters = report["clusters"].as_array().expect("clusters array");
    assert_eq!(clusters.len(), 1, "fixture returns one canned cluster: {report}");
    let cluster = &clusters[0];
    assert_eq!(cluster["fingerprint"], FIXTURE_DEFAULT_FP);
    assert_eq!(cluster["occurrences"], 2);
    assert_eq!(cluster["screens"], serde_json::json!(["home", "search"]));
    assert_eq!(cluster["bound-slug"], Value::Null);
    assert_eq!(cluster["evidence"]["region"], "footer");
    assert!(
        dispatch_fixture_adapter_dir().join("adapter.wasm").is_file(),
        "fixture adapter.wasm must be checked in"
    );
}

#[test]
fn report_absent_baseline_is_empty() {
    let tmp = bind_project();
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "catalog", "infer", "--phase", "report"])
        .assert()
        .success();

    let report = parse_json(&assert.get_output().stdout);
    assert_eq!(report["version"], 1);
    assert_eq!(report["clusters"], serde_json::json!([]));
    assert_eq!(report["unmatched-parts"], serde_json::json!([]));
    assert!(!ComponentsCatalog::path_in(tmp.path()).exists(), "no catalog written");
}

#[test]
fn bind_dry_run_prints_diff_without_writing() {
    let tmp = bind_project();
    let bindings = write_bindings(
        tmp.path(),
        "bindings:\n  a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1: tab-bar\n",
    );
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "catalog", "infer", "--phase", "bind"])
        .arg("--bindings")
        .arg(&bindings)
        .arg("--dry-run")
        .assert()
        .success();

    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["dry-run"], true);
    assert_eq!(body["added"], serde_json::json!(["tab-bar"]));
    assert!(!ComponentsCatalog::path_in(tmp.path()).exists(), "dry-run writes nothing");
}

#[test]
fn bind_writes_supplied_slug() {
    let tmp = bind_project();
    let bindings = write_bindings(
        tmp.path(),
        "bindings:\n  a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1: tab-bar\n",
    );
    specify_cmd()
        .current_dir(tmp.path())
        .args(["catalog", "infer", "--phase", "bind"])
        .arg("--bindings")
        .arg(&bindings)
        .assert()
        .success();

    let catalog = load_catalog(tmp.path()).expect("catalog written");
    assert_eq!(catalog.status_of("tab-bar"), Some(ComponentStatus::Confirmed));
    assert_eq!(
        catalog.components.get("tab-bar").and_then(|e| e.fingerprint.clone()),
        Some("a1".repeat(32)),
        "bind persists the fingerprint so a later report can echo the slug"
    );
}

#[test]
fn bind_rejects_non_hex_fingerprint() {
    let tmp = bind_project();
    // Key is not 64-char lowercase hex; binding would otherwise persist a
    // catalog the schema-validated `ComponentsCatalog::load` later rejects.
    let bindings = write_bindings(tmp.path(), "bindings:\n  not-a-fingerprint: tab-bar\n");
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "catalog", "infer", "--phase", "bind"])
        .arg("--bindings")
        .arg(&bindings)
        .assert()
        .failure();

    assert_eq!(
        parse_stderr(&assert.get_output().stderr, tmp.path())["error"],
        "catalog-bindings-malformed",
    );
    assert!(!ComponentsCatalog::path_in(tmp.path()).exists(), "a rejected bind writes nothing");
}

#[test]
fn bind_rejects_non_kebab_slug() {
    let tmp = bind_project();
    // Valid fingerprint, but `TabBar` violates the catalog's kebab-case
    // slug pattern — caught before any write.
    let bindings =
        write_bindings(tmp.path(), &format!("bindings:\n  {}: TabBar\n", "a1".repeat(32)));
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "catalog", "infer", "--phase", "bind"])
        .arg("--bindings")
        .arg(&bindings)
        .assert()
        .failure();

    assert_eq!(
        parse_stderr(&assert.get_output().stderr, tmp.path())["error"],
        "catalog-bindings-malformed",
    );
    assert!(!ComponentsCatalog::path_in(tmp.path()).exists(), "a rejected bind writes nothing");
}

/// Bind a fixture fingerprint, then re-run `report` and assert
/// `populate_bound_slugs` fills `bound-slug` from the catalog.
#[test]
fn report_echoes_bound_slug_after_bind() {
    let (tmp, cache) = report_project(REPEATED_GROUP_BASELINE);

    let bindings =
        write_bindings(tmp.path(), &format!("bindings:\n  {FIXTURE_DEFAULT_FP}: shared-footer\n"));
    specify_cmd()
        .current_dir(tmp.path())
        .args(["catalog", "infer", "--phase", "bind"])
        .arg("--bindings")
        .arg(&bindings)
        .assert()
        .success();

    let second = specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["--format", "json", "catalog", "infer", "--phase", "report"])
        .assert()
        .success();
    let report = parse_json(&second.get_output().stdout);
    assert_eq!(
        report["clusters"][0]["bound-slug"], "shared-footer",
        "report echoes the slug bound to this fingerprint: {report}"
    );
}

#[test]
fn bind_preserves_rejected() {
    let tmp = bind_project();
    let mut seed = ComponentsCatalog::empty();
    seed.components.insert(
        "tab-bar".to_string(),
        specify_workflow::design_system::ComponentEntry {
            status: ComponentStatus::Rejected,
            description: Some("operator says no".to_string()),
            fingerprint: None,
        },
    );
    seed.save(tmp.path()).expect("seed catalog");

    let bindings = write_bindings(
        tmp.path(),
        "bindings:\n  a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1: tab-bar\n",
    );
    specify_cmd()
        .current_dir(tmp.path())
        .args(["catalog", "infer", "--phase", "bind"])
        .arg("--bindings")
        .arg(&bindings)
        .assert()
        .success();

    let catalog = load_catalog(tmp.path()).expect("catalog present");
    assert_eq!(catalog.status_of("tab-bar"), Some(ComponentStatus::Rejected));
    assert_eq!(catalog.components.len(), 1, "rejected entry not re-added as a second entry");
}

#[test]
fn bind_leaves_existing_confirmed_untouched() {
    let tmp = bind_project();
    let mut seed = ComponentsCatalog::empty();
    seed.components.insert(
        "tab-bar".to_string(),
        specify_workflow::design_system::ComponentEntry {
            status: ComponentStatus::Confirmed,
            description: Some("original".to_string()),
            fingerprint: None,
        },
    );
    seed.save(tmp.path()).expect("seed catalog");

    let bindings = write_bindings(
        tmp.path(),
        "bindings:\n  a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1:\n    slug: tab-bar\n    description: replacement\n",
    );
    specify_cmd()
        .current_dir(tmp.path())
        .args(["catalog", "infer", "--phase", "bind"])
        .arg("--bindings")
        .arg(&bindings)
        .assert()
        .success();

    let catalog = load_catalog(tmp.path()).expect("catalog present");
    assert_eq!(
        catalog.components.get("tab-bar").and_then(|e| e.description.clone()),
        Some("original".to_string()),
        "existing confirmed description is untouched"
    );
}

#[test]
fn bind_suffixes_slug_collision() {
    let tmp = bind_project();
    // Two distinct fingerprints handed the same bare slug. The
    // lexicographically-first (a1…) keeps `card-row`; the later (b2…) is
    // suffixed with its 8-char fingerprint prefix.
    let bindings = write_bindings(
        tmp.path(),
        "bindings:\n  a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1: card-row\n  b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2b2: card-row\n",
    );
    specify_cmd()
        .current_dir(tmp.path())
        .args(["catalog", "infer", "--phase", "bind"])
        .arg("--bindings")
        .arg(&bindings)
        .assert()
        .success();

    let catalog = load_catalog(tmp.path()).expect("catalog present");
    assert_eq!(catalog.status_of("card-row"), Some(ComponentStatus::Confirmed));
    assert_eq!(catalog.status_of("card-row-b2b2b2b2"), Some(ComponentStatus::Confirmed));
    assert_eq!(catalog.components.len(), 2, "both fingerprints bound under distinct slugs");
}

/// A pinned operator part is projected into the catalog via `part_projections`
/// host wiring (no skill `--bindings` file).
#[test]
fn bind_projects_matched_pin_via_fixture() {
    let (tmp, cache) = report_project(SINGLE_FOOTER_BASELINE);
    write_parts(tmp.path(), PRIMARY_NAV_PART);

    specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["catalog", "infer", "--phase", "bind"])
        .assert()
        .success();

    let catalog = load_catalog(tmp.path()).expect("catalog written");
    assert_eq!(catalog.status_of("primary-nav"), Some(ComponentStatus::Confirmed));
    assert_eq!(
        catalog.components.get("primary-nav").and_then(|e| e.fingerprint.clone()),
        Some(FIXTURE_PINNED_FP.to_string()),
        "part projection records the fixture pinned fingerprint"
    );
}

/// The operator slug is the first-writer for its fingerprint: a skill
/// binding handed the same bare name under a *different* fingerprint is
/// suffixed `slug-<fp-prefix>` by the slug-uniqueness guard.
#[test]
fn bind_operator_part_wins_slug_over_skill() {
    let (tmp, cache) = report_project(SINGLE_FOOTER_BASELINE);
    write_parts(
        tmp.path(),
        "version: 1
parts:
  card-row:
    group:
      items:
        - icon-button: {}
        - icon-button: {}
",
    );

    let bindings =
        write_bindings(tmp.path(), &format!("bindings:\n  {FIXTURE_UNPINNED_FP}: card-row\n"));
    specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["catalog", "infer", "--phase", "bind"])
        .arg("--bindings")
        .arg(&bindings)
        .assert()
        .success();

    let catalog = load_catalog(tmp.path()).expect("catalog present");
    assert_eq!(
        catalog.status_of("card-row"),
        Some(ComponentStatus::Confirmed),
        "the operator part keeps the bare slug"
    );
    let suffixed = format!("card-row-{}", &FIXTURE_UNPINNED_FP[..8]);
    assert_eq!(
        catalog.status_of(&suffixed),
        Some(ComponentStatus::Confirmed),
        "the skill binding under a different fingerprint is suffixed: {:?}",
        catalog.components.keys().collect::<Vec<_>>()
    );
}

#[test]
fn bind_is_idempotent_for_a_fixed_map() {
    let tmp = bind_project();
    let bindings = write_bindings(
        tmp.path(),
        "bindings:\n  a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1a1: tab-bar\n",
    );
    let run = || {
        specify_cmd()
            .current_dir(tmp.path())
            .args(["catalog", "infer", "--phase", "bind"])
            .arg("--bindings")
            .arg(&bindings)
            .assert()
            .success();
    };
    run();
    let first = load_catalog(tmp.path()).expect("catalog present");
    run();
    let second = load_catalog(tmp.path()).expect("catalog present");
    assert_eq!(first, second, "re-running bind with the same map is a no-op");
}

#[test]
fn report_rejects_missing_catalog_infer() {
    let tmp = tempdir().expect("tempdir");
    let project = tmp.path();
    let adapter = project.join("adapters/targets/omnia");
    let briefs = adapter.join("briefs");
    fs::create_dir_all(project.join(".specify/specs")).expect("create specs");
    fs::create_dir_all(&briefs).expect("create briefs");
    fs::write(
        project.join(".specify/project.yaml"),
        "name: catalog-test\nadapter: omnia\nrules: {}\n",
    )
    .expect("write project.yaml");
    fs::write(
        adapter.join("adapter.yaml"),
        "name: omnia\nversion: 1.0.0\naxis: target\nexecution: agent\nbriefs:\n  shape: briefs/shape.md\n  build: briefs/build.md\n  merge: briefs/merge.md\nextension:\n  name: omnia\n  permissions:\n    read:\n      - $PROJECT_DIR/.specify\n    write: []\ndescription: Test omnia adapter\n",
    )
    .expect("write adapter.yaml");
    for op in ["shape", "build", "merge"] {
        fs::write(
            briefs.join(format!("{op}.md")),
            format!("---\nid: {op}\ndescription: {op} brief\n---\n"),
        )
        .expect("write brief");
    }
    fs::write(project.join(".specify/specs/composition.yaml"), SINGLE_FOOTER_BASELINE)
        .expect("write composition.yaml");

    let assert = specify_cmd()
        .current_dir(project)
        .args(["--format", "json", "catalog", "infer", "--phase", "report"])
        .assert()
        .failure();

    let stderr = parse_stderr(&assert.get_output().stderr, project);
    assert_eq!(
        stderr["error"], "catalog-infer-unsupported",
        "catalog infer must refuse adapters that omit catalog.infer: {stderr}"
    );
}

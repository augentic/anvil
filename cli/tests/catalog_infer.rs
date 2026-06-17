//! Integration tests for `specify catalog infer`.
//!
//! These tests assert the host **mechanism** — the report shape, and
//! the bind guards (stability / uniqueness / no-overwrite) — and
//! never that a specific English name like `tab-bar` emerges, because
//! naming is the build skill's judgement, not the CLI's. Where
//! a bound name is needed, the test supplies a fixed `{ fingerprint →
//! slug }` bindings map standing in for the agent's decision.
//!
//! The single `report` test that dispatches the real `vectis infer`
//! tool is skipped when the WASM artifact is absent (build it with
//! `cargo make vectis-wasm`); the bind tests are pure host bookkeeping
//! and need no tool.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{parse_json, parse_stderr, repo_root, specify_cmd};
use serde_json::Value;
use specify_workflow::adapter::ADAPTER_WASM_FILENAME;
use specify_workflow::design_system::{ComponentStatus, ComponentsCatalog};
use tempfile::{TempDir, tempdir};

fn vectis_wasm() -> PathBuf {
    repo_root().join("target/vectis-wasi-tools/release/vectis.wasm")
}

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
        "name: catalog-test\nadapter: vectis\nrules: {}\n",
    )
    .expect("write project.yaml");
    tmp
}

/// Scaffold a project that declares the `vectis` WASI extension via the
/// adapter's singular `extension:` object with read access to `.specify/`,
/// stages the committed `adapter.wasm`, and writes a composition baseline
/// — everything the `report` phase needs to dispatch the real tool.
/// Returns the project tempdir and the extensions cache dir.
fn report_project(baseline: &str) -> (TempDir, PathBuf) {
    let tmp = tempdir().expect("tempdir");
    let project = tmp.path();
    let adapter = project.join("adapters/targets/vectis");
    let briefs = adapter.join("briefs");
    fs::create_dir_all(project.join(".specify/specs")).expect("create specs");
    fs::create_dir_all(&briefs).expect("create briefs");

    fs::write(
        project.join(".specify/project.yaml"),
        "name: catalog-test\nadapter: vectis\nrules: {}\n",
    )
    .expect("write project.yaml");
    fs::write(
        adapter.join("adapter.yaml"),
        "name: vectis\nversion: 1.0.0\naxis: target\nexecution: agent\nbriefs:\n  shape: briefs/shape.md\n  build: briefs/build.md\n  merge: briefs/merge.md\nextension:\n  name: vectis\n  permissions:\n    read:\n      - $PROJECT_DIR/.specify\n    write: []\ndescription: Test vectis adapter\n",
    )
    .expect("write adapter.yaml");
    for op in ["shape", "build", "merge"] {
        fs::write(
            briefs.join(format!("{op}.md")),
            format!("---\nid: {op}\ndescription: {op} brief\n---\n"),
        )
        .expect("write brief");
    }

    // RFC-48 D11: the resolved adapter declares its WASI extension via the
    // singular `extension:` object; the host sources the component from the
    // committed `adapter.wasm`, not a retired `tools.yaml` sidecar.
    fs::copy(vectis_wasm(), adapter.join(ADAPTER_WASM_FILENAME)).expect("stage adapter.wasm");

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
fn report_clusters_repeated_group() {
    let wasm = vectis_wasm();
    if !wasm.is_file() {
        eprintln!(
            "skipping: vectis WASM not found at {}; run `cargo make vectis-wasm`",
            wasm.display()
        );
        return;
    }

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
    assert_eq!(clusters.len(), 1, "exactly one above-threshold cluster: {report}");
    let cluster = &clusters[0];
    assert_eq!(cluster["occurrences"], 2);
    assert_eq!(cluster["screens"], serde_json::json!(["home", "search"]));
    assert!(
        cluster["fingerprint"].as_str().is_some_and(|f| f.len() == 64),
        "fingerprint is a 64-char sha256 hex string: {cluster}"
    );
    assert_eq!(cluster["bound-slug"], Value::Null);
    assert_eq!(cluster["evidence"]["region"], "footer");
}

/// A single baseline screen plus one candidate-cache entry carrying a
/// structurally identical group on a different provenance screen cluster
/// to one shared component at the default threshold: the
/// cache supplies the second screen the baseline has not yet accumulated.
#[test]
fn report_clusters_with_candidate_cache() {
    let wasm = vectis_wasm();
    if !wasm.is_file() {
        eprintln!("skipping: vectis WASM not found at {}", wasm.display());
        return;
    }

    let single_screen = "version: 1
screens:
  home:
    name: Home
    footer:
      - group:
          items:
            - icon-button: { bind: home, event: Navigate(Home) }
            - icon-button: { bind: search, event: Navigate(Search) }
";
    let (tmp, cache) = report_project(single_screen);

    // Without the cache, the lone baseline group is below threshold.
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["--format", "json", "catalog", "infer", "--phase", "report"])
        .assert()
        .success();
    let report = parse_json(&assert.get_output().stdout);
    assert_eq!(report["clusters"], serde_json::json!([]), "single screen is below threshold");

    // Seed a candidate-cache entry for a different screen with the same
    // structure; the verb now proposes the shared component.
    let candidate_dir =
        tmp.path().join(".specify/.cache/component-candidates/checkout-slice/checkout");
    fs::create_dir_all(&candidate_dir).expect("create candidate cache dir");
    fs::write(
        candidate_dir.join("footer.0.yaml"),
        "candidate_component: nav-footer
region: footer
group:
  items:
    - icon-button: { bind: home, event: Navigate(Home) }
    - icon-button: { bind: orders, event: Navigate(Orders) }
",
    )
    .expect("write candidate cache entry");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["--format", "json", "catalog", "infer", "--phase", "report"])
        .assert()
        .success();
    let report = parse_json(&assert.get_output().stdout);
    let clusters = report["clusters"].as_array().expect("clusters array");
    assert_eq!(clusters.len(), 1, "cache + baseline group cluster as one candidate: {report}");
    let cluster = &clusters[0];
    assert_eq!(cluster["occurrences"], 2);
    assert_eq!(cluster["screens"], serde_json::json!(["checkout", "home"]));
    assert_eq!(cluster["bound-slug"], Value::Null);
    assert_eq!(
        cluster["evidence"]["candidate-names"],
        serde_json::json!(["nav-footer"]),
        "the stage-6 label hint surfaces as non-authoritative evidence"
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

/// End-to-end run-to-run stability: bind a fingerprint, then
/// re-run `report` against the same baseline and assert the cluster now
/// carries the bound slug. Uses the real tool, so it is skipped when the
/// WASM artifact is absent.
#[test]
fn report_echoes_bound_slug_after_bind() {
    let wasm = vectis_wasm();
    if !wasm.is_file() {
        eprintln!("skipping: vectis WASM not found at {}", wasm.display());
        return;
    }

    let (tmp, cache) = report_project(REPEATED_GROUP_BASELINE);

    // First report: capture the cluster's fingerprint (the agent would
    // name it here; the test stands in with a fixed slug).
    let first = specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["--format", "json", "catalog", "infer", "--phase", "report"])
        .assert()
        .success();
    let report = parse_json(&first.get_output().stdout);
    let fingerprint =
        report["clusters"][0]["fingerprint"].as_str().expect("fingerprint").to_string();

    // Bind that fingerprint to a fixed slug.
    let bindings =
        write_bindings(tmp.path(), &format!("bindings:\n  {fingerprint}: shared-footer\n"));
    specify_cmd()
        .current_dir(tmp.path())
        .args(["catalog", "infer", "--phase", "bind"])
        .arg("--bindings")
        .arg(&bindings)
        .assert()
        .success();

    // Second report: the same cluster now echoes the bound slug.
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

/// A pinned operator part matching one baseline group is projected into
/// the catalog as `confirmed` below the default threshold (promotion
/// authority) with the operator slug and the part's
/// read-time fingerprint recorded. Uses the real tool (bind runs `infer`
/// with `--parts`), so it is skipped when the WASM artifact is absent.
#[test]
fn bind_projects_matched_pin() {
    let wasm = vectis_wasm();
    if !wasm.is_file() {
        eprintln!("skipping: vectis WASM not found at {}", wasm.display());
        return;
    }
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
    assert!(
        catalog.components.get("primary-nav").and_then(|e| e.fingerprint.as_ref()).is_some(),
        "a part-bound entry records the fingerprint so a later report echoes it: {:?}",
        catalog.components.get("primary-nav")
    );
}

/// A pinned part matching zero baseline groups is reported `part-unmatched`
/// and never projected into the catalog. The
/// report is informational and `bind` still succeeds.
#[test]
fn bind_reports_unmatched_pin() {
    let wasm = vectis_wasm();
    if !wasm.is_file() {
        eprintln!("skipping: vectis WASM not found at {}", wasm.display());
        return;
    }
    let baseline = "version: 1
screens:
  home:
    name: Home
    body:
      - text: { content: hi }
";
    let (tmp, cache) = report_project(baseline);
    write_parts(
        tmp.path(),
        "version: 1
parts:
  primary-nav:
    group:
      items:
        - icon-button: {}
        - icon-button: {}
        - icon-button: {}
",
    );

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["--format", "json", "catalog", "infer", "--phase", "bind"])
        .assert()
        .success();

    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["unmatched-parts"], serde_json::json!(["primary-nav"]));
    assert_eq!(body["added"], serde_json::json!([]));
    assert!(
        !ComponentsCatalog::path_in(tmp.path()).exists(),
        "an unmatched pin scaffolds no catalog entry"
    );
}

/// A part slug equal to an existing `rejected` catalog entry stays
/// suppressed: the projection's `upsert_bound` is a no-op against a
/// rejected slug, so the part is not factored.
#[test]
fn bind_part_does_not_override_rejected() {
    let wasm = vectis_wasm();
    if !wasm.is_file() {
        eprintln!("skipping: vectis WASM not found at {}", wasm.display());
        return;
    }
    let (tmp, cache) = report_project(SINGLE_FOOTER_BASELINE);
    write_parts(tmp.path(), PRIMARY_NAV_PART);

    let mut seed = ComponentsCatalog::empty();
    seed.components.insert(
        "primary-nav".to_string(),
        specify_workflow::design_system::ComponentEntry {
            status: ComponentStatus::Rejected,
            description: Some("operator says no".to_string()),
            fingerprint: None,
        },
    );
    seed.save(tmp.path()).expect("seed catalog");

    specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["catalog", "infer", "--phase", "bind"])
        .assert()
        .success();

    let catalog = load_catalog(tmp.path()).expect("catalog present");
    assert_eq!(catalog.status_of("primary-nav"), Some(ComponentStatus::Rejected));
    assert_eq!(catalog.components.len(), 1, "rejected pin is not factored as a second entry");
}

/// The operator slug is the first-writer for its fingerprint: a skill
/// binding handed the same bare name under a *different* fingerprint is
/// suffixed `slug-<fp-prefix>` by the slug-uniqueness guard, while the
/// operator part keeps the bare slug.
#[test]
fn bind_operator_part_wins_slug_over_skill() {
    let wasm = vectis_wasm();
    if !wasm.is_file() {
        eprintln!("skipping: vectis WASM not found at {}", wasm.display());
        return;
    }
    // `home` carries the part-matched footer group (one occurrence,
    // promoted by the pin) and a 3-text body group repeated on `search`
    // (clusters on its own at threshold 2).
    let baseline = "version: 1
screens:
  home:
    name: Home
    footer:
      - group:
          items:
            - icon-button: { bind: home, event: Navigate(Home) }
            - icon-button: { bind: search, event: Navigate(Search) }
    body:
      - group:
          items:
            - text: {}
            - text: {}
            - text: {}
  search:
    name: Search
    body:
      - group:
          items:
            - text: {}
            - text: {}
            - text: {}
";
    let (tmp, cache) = report_project(baseline);
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

    // Report to discover the unpinned body cluster's fingerprint, which
    // the skill will (deliberately) try to also name `card-row`.
    let assert = specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["--format", "json", "catalog", "infer", "--phase", "report"])
        .assert()
        .success();
    let report = parse_json(&assert.get_output().stdout);
    let clusters = report["clusters"].as_array().expect("clusters");
    let body_fp = clusters
        .iter()
        .find(|c| c["bound-slug"].is_null())
        .and_then(|c| c["fingerprint"].as_str())
        .expect("an unpinned body cluster")
        .to_string();

    let bindings = write_bindings(tmp.path(), &format!("bindings:\n  {body_fp}: card-row\n"));
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
    let suffixed = format!("card-row-{}", &body_fp[..8]);
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

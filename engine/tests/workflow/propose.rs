//! `specify plan propose` CLI tests: dry-run request envelopes,
//! `--from` happy paths, journal tail, negative gates, and re-propose
//! semantics, plus the propose-only seeds and helpers.

use crate::support::*;

// -- propose seeds ----------------------------------------------------

/// N=1 plan: a single `intent` source, no slices yet (replaceable).
const PROPOSE_PLAN_N1: &str = "\
name: demo
sources:
  intent:
    adapter: intent
    value: \"Fix a typo in user.rs.\"
slices: []
";

/// N=1 surveyed inventory: one `intent` lead.
const PROPOSE_DISCOVERY_N1: &str = "\
## Lead inventory

### intent:fix-typo

- lead: fix-typo
- source: intent
- synopsis: Fix a typo in user.rs.
";

/// N=1 agent response: omits `project` (kernel auto-binds the sole
/// project) and carries the explicit slice `name`.
const PROPOSE_RESPONSE_N1: &str = r#"{
  "version": 1,
  "kind": "response",
  "slices": [
    { "name": "fix-typo", "sources": [{ "source": "intent", "lead": "fix-typo" }] }
  ]
}"#;

/// Workspace registry with two projects bound to different target adapters —
/// the topology the fan-out response binds against.
const PROPOSE_REGISTRY_WORKSPACE: &str = "\
version: 1
projects:
  - name: identity-contracts
    url: git@github.com:org/identity-contracts.git
    adapter: contracts@1.0.0
    description: Versioned API contracts crate for the identity domain.
  - name: identity-service
    url: git@github.com:org/identity-service.git
    adapter: omnia@1.0.0
    description: Omnia identity service implementing auth and password flows.
";

/// Workspace surveyed inventory: four leads across `docs` + `legacy` (the
/// proposal-schema envelope example, in document order).
const PROPOSE_DISCOVERY_WORKSPACE: &str = "\
## Lead inventory

### docs:identity-api

- lead: identity-api
- source: docs
- synopsis: Identity API contract for authentication and account access.

### legacy:identity-api

- lead: identity-api
- source: legacy
- synopsis: Legacy identity endpoints.

### docs:password-reset

- lead: password-reset
- source: docs
- synopsis: Users can request a password reset email.

### legacy:reset-password

- lead: reset-password
- source: legacy
- synopsis: Legacy reset-password flow.
";

/// Committed `.specify/topology.lock` for the workspace fixture —
/// the projection `workspace sync` would derive from each member
/// project's `project.yaml`. Descriptions mirror the registry seeds so
/// the request envelope's `projects[]` stays the authoritative shape.
const PROPOSE_TOPOLOGY_WORKSPACE: &str = "\
version: 1
projects:
  - name: identity-contracts
    target: contracts@1.0.0
    description: Versioned API contracts crate for the identity domain.
  - name: identity-service
    target: omnia@1.0.0
    description: Omnia identity service implementing auth and password flows.
";

/// Workspace plan declaring the two surveyed source keys, no slices yet.
const PROPOSE_PLAN_WORKSPACE: &str = "\
name: identity-revamp
sources:
  docs:
    adapter: documentation
    path: ./docs
  legacy:
    adapter: typescript
    path: ./legacy
slices: []
";

/// Multi-source fan-out response (the proposal-schema envelope
/// example): the `identity-api` lead is referenced by two slices
/// (`identity-contracts` + `identity-service`, joined by `depends-on`);
/// `password-reset` is a single slice matched across sources by summary.
const PROPOSE_RESPONSE_FANOUT: &str = r#"{
  "version": 1,
  "kind": "response",
    "slices": [
    {
      "name": "identity-contracts",
      "sources": [
        { "source": "docs", "lead": "identity-api" },
        { "source": "legacy", "lead": "identity-api" }
      ],
      "project": "identity-contracts",
      "rationale": "identity API surface matched by shared slug across docs + legacy"
    },
    {
      "name": "identity-service",
      "sources": [
        { "source": "docs", "lead": "identity-api" },
        { "source": "legacy", "lead": "identity-api" }
      ],
      "project": "identity-service",
      "depends-on": ["identity-contracts"]
    },
    {
      "name": "password-reset",
      "sources": [
        { "source": "docs", "lead": "password-reset" },
        { "source": "legacy", "lead": "reset-password" }
      ],
      "project": "identity-service",
      "rationale": "password-reset (docs) and reset-password (legacy) are the same flow by synopsis judgment"
    }
  ]
}"#;

// -- propose helpers --------------------------------------------------

/// Build a minimal `discovery.md` body with one `### source:lead` block
/// per `(source, lead)` pair — mirrors the kernel unit-test
/// seeding so negative fixtures stay one-liners.
fn discovery_doc(leads: &[(&str, &str)]) -> String {
    use std::fmt::Write as _;
    let mut body = String::from("## Lead inventory\n\n");
    for (source, lead) in leads {
        let _ = write!(
            body,
            "### {source}:{lead}\n\n\
             - lead: {lead}\n\
             - source: {source}\n\
             - synopsis: {lead} synopsis.\n\n",
        );
    }
    body
}

fn seed_discovery(root: &Path, body: &str) {
    fs::write(root.join("discovery.md"), body).expect("write discovery.md");
}

/// Write a `--from` response file under `root`, returning its path.
fn write_response(root: &Path, body: &str) -> PathBuf {
    let path = root.join("response.json");
    fs::write(&path, body).expect("write response.json");
    path
}

/// Scaffold a workspace project in a fresh tempdir, seeding
/// `registry.yaml`, `discovery.md`, and `plan.yaml`.
fn workspace_project(registry: &str, discovery: &str, plan: &str) -> TempDir {
    let tmp = tempdir().expect("tempdir");
    init_workspace(&tmp, "platform-workspace");
    fs::write(tmp.path().join("registry.yaml"), registry).expect("write registry.yaml");
    seed_discovery(tmp.path(), discovery);
    fs::write(tmp.path().join("plan.yaml"), plan).expect("write plan.yaml");
    // Workspace plan-time topology reads the committed cache, not the
    // registry. Seed the projection `workspace sync` would produce for
    // the remote members (which a unit test cannot materialise).
    fs::write(tmp.path().join(".specify/topology.lock"), PROPOSE_TOPOLOGY_WORKSPACE)
        .expect("write topology.lock");
    tmp
}

/// Run `plan propose --from <body>` expecting an exit-2 abort and
/// return the parsed `--format json` stderr envelope.
fn propose_from_stderr(root: &Path, body: &str) -> Value {
    let response = write_response(root, body);
    let assert = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "plan", "propose", "--from"])
        .arg(&response)
        .assert()
        .failure();
    assert_eq!(
        assert.get_output().status.code(),
        Some(2),
        "every propose --from invariant aborts at exit 2"
    );
    parse_stderr(&assert.get_output().stderr, root)
}

/// Run `plan propose --from <body>` expecting success and return the
/// parsed `--format json` stdout summary.
fn propose_from_ok(root: &Path, body: &str) -> Value {
    let response = write_response(root, body);
    let assert = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "plan", "propose", "--from"])
        .arg(&response)
        .assert()
        .success();
    parse_stdout(&assert.get_output().stdout, root)
}

// -- dry-run request envelope goldens --------------------------------

#[test]
fn propose_dry_run_n1_request_golden() {
    // N=1: the sole regular project is synthesised from `project.yaml`
    // (`test-proj` → `omnia@1.0.0`); one `intent` lead surfaces.
    let project = Project::init();
    project.seed_plan(PROPOSE_PLAN_N1);
    seed_discovery(project.root(), PROPOSE_DISCOVERY_N1);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "propose", "--dry-run"])
        .assert()
        .success();
    let actual = parse_stdout(&assert.get_output().stdout, project.root());

    assert_eq!(actual["kind"], "request");
    assert_eq!(actual["projects"].as_array().expect("projects").len(), 1);
    assert_eq!(actual["projects"][0]["name"], "test-proj");
    assert_eq!(actual["projects"][0]["target"], "omnia@1.0.0");
    assert_eq!(actual["leads"].as_array().expect("leads").len(), 1);
    assert_eq!(actual["leads"][0]["source"], "intent");
    assert_eq!(actual["leads"][0]["lead"], "fix-typo");

    // The plan is untouched by --dry-run.
    assert_eq!(fs::read_to_string(project.plan_path()).expect("read plan"), PROPOSE_PLAN_N1);

    // The plan scratch lane is recreated empty for the response envelope.
    let lane = project.root().join(".specify/scratch/plan");
    assert!(lane.is_dir(), "dry-run must create the plan scratch lane");
    assert_eq!(
        fs::read_dir(&lane).expect("read lane").count(),
        0,
        "dry-run must leave the plan scratch lane empty"
    );

    assert_golden("propose-dry-run-n1-request.json", actual);
}

#[test]
fn propose_dry_run_clears_stale_response() {
    // A prior run's response envelope must never survive a fresh
    // dry-run, so `--from` cannot consume a stale grouping.
    let project = Project::init();
    project.seed_plan(PROPOSE_PLAN_N1);
    seed_discovery(project.root(), PROPOSE_DISCOVERY_N1);
    let lane = project.root().join(".specify/scratch/plan");
    fs::create_dir_all(&lane).expect("mkdir plan lane");
    fs::write(lane.join("propose-response.json"), "{}").expect("seed stale response");

    specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "propose", "--dry-run"])
        .assert()
        .success();

    assert!(
        !lane.join("propose-response.json").exists(),
        "dry-run must drop a stale response envelope"
    );
}

// -- `--from` happy-path goldens -------------------------------------

#[test]
fn propose_from_n1_auto_bind_golden() {
    let project = Project::init();
    project.seed_plan(PROPOSE_PLAN_N1);
    seed_discovery(project.root(), PROPOSE_DISCOVERY_N1);

    let actual = propose_from_ok(project.root(), PROPOSE_RESPONSE_N1);
    assert_eq!(actual["plan"]["name"], "demo");
    assert_eq!(actual["slice-count"], 1);
    assert_eq!(actual["slice-names"], serde_json::json!(["fix-typo"]));
    assert_golden("propose-from-n1-summary.json", actual);

    // The projected plan: one slice, target derived from the
    // auto-bound project, structured source binding.
    let plan = Plan::load(&project.plan_path()).expect("load plan");
    assert_eq!(plan.entries.len(), 1);
    let entry = &plan.entries[0];
    assert_eq!(entry.name, "fix-typo");
    // Target is no longer stored on the slice; the bound project is the
    // sole binding and the target resolves from it on demand.
    assert_eq!(entry.project.as_deref(), Some("test-proj"));
    assert_eq!(entry.sources.len(), 1);
    assert_eq!(entry.sources[0].source(), "intent");
    assert_eq!(entry.sources[0].lead("fix-typo"), "fix-typo");
}

// -- journal tail -----------------------------------------------------

#[test]
fn propose_from_emits_single_journal_tail() {
    let tmp = workspace_project(
        PROPOSE_REGISTRY_WORKSPACE,
        PROPOSE_DISCOVERY_WORKSPACE,
        PROPOSE_PLAN_WORKSPACE,
    );
    let response = write_response(tmp.path(), PROPOSE_RESPONSE_FANOUT);
    specify_cmd()
        .current_dir(tmp.path())
        .args(["plan", "propose", "--from"])
        .arg(&response)
        .assert()
        .success();

    let raw = fs::read_to_string(tmp.path().join(".specify/journal.jsonl")).expect("read journal");
    let events: Vec<Value> = raw
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("journal line is JSON"))
        .collect();
    assert_eq!(events.len(), 1, "exactly one reconcile event fires, got:\n{events:#?}");

    // The former agent/completed pair folded into one
    // `plan.reconcile.completed` event carrying the slice names in order.
    let completed = &events[0];
    assert_eq!(completed["event"], "plan.reconcile.completed");
    assert_eq!(completed["payload"]["plan-name"], "identity-revamp");
    assert_eq!(completed["payload"]["slice-count"], 3);
    assert_eq!(
        completed["payload"]["slice-names"],
        serde_json::json!(["identity-contracts", "identity-service", "password-reset"])
    );
}

// -- platform bootstrap (default-on reconciliation) --------------------

/// N=1 plan/discovery/response for a single feature lead bound to the
/// sole regular project — `propose --from` augments with bootstrap slices
/// when `project.yaml.platforms` is non-empty.
const PLATFORM_PLAN: &str = "\
name: platform-app
sources:
  intent:
    adapter: intent
    value: \"Add a feature.\"
slices: []
";

const PLATFORM_DISCOVERY: &str = "\
## Lead inventory

### intent:add-feature

- lead: add-feature
- source: intent
- synopsis: Add a feature.
";

const PLATFORM_RESPONSE: &str = r#"{"version":1,"kind":"response","slices":[{"name":"add-feature","sources":[{"source":"intent","lead":"add-feature"}]}]}"#;

/// In-repo vectis stub target adapter (declares `platforms.required`).
fn vectis_stub_dir() -> PathBuf {
    repo_root().join("tests/fixtures/adapters/targets/vectis-stub")
}

/// Scaffold a single-project tree backed by a Vectis target with
/// `--platforms <set>`, then seed the propose inputs (plan + discovery).
fn platform_project(platforms: &str) -> TempDir {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    let adapter = root.join("adapters/targets/vectis");
    copy_dir(&vectis_stub_dir(), &adapter);

    let adapter_yaml =
        fs::read_to_string(adapter.join("adapter.yaml")).expect("read vectis-stub adapter.yaml");
    fs::write(
        adapter.join("adapter.yaml"),
        adapter_yaml.replace("name: vectis-stub", "name: vectis"),
    )
    .expect("patch adapter name to vectis");

    specify_cmd()
        .current_dir(root)
        .args(["init"])
        .arg(&adapter)
        .args(["--name", "platform-app", "--platforms", platforms])
        .assert()
        .success();
    fs::write(root.join("plan.yaml"), PLATFORM_PLAN).expect("write plan.yaml");
    seed_discovery(root, PLATFORM_DISCOVERY);
    tmp
}

/// Read `.specify/journal.jsonl` as one parsed `Value` per line.
fn journal_events(root: &Path) -> Vec<Value> {
    let raw = fs::read_to_string(root.join(".specify/journal.jsonl")).expect("read journal");
    raw.lines()
        .filter(|l| !l.is_empty())
        .map(|l| serde_json::from_str(l).expect("journal line is JSON"))
        .collect()
}

/// Run `plan propose --from <body>` expecting success and return the
/// parsed `--format json` summary.
fn propose_reconcile_ok(root: &Path, body: &str) -> Value {
    let response = write_response(root, body);
    let assert = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "plan", "propose", "--from"])
        .arg(&response)
        .assert()
        .success();
    parse_stdout(&assert.get_output().stdout, root)
}

#[test]
fn reconcile_greenfield_app_foundation() {
    // A greenfield tree (no shells on disk) with every supported
    // platform declared collapses to one `app-foundation` bootstrap
    // slice, prepended ahead of the feature slice.
    let tmp = platform_project("core,ios,android");

    let summary = propose_reconcile_ok(tmp.path(), PLATFORM_RESPONSE);
    assert_eq!(
        summary["slice-names"],
        serde_json::json!(["app-foundation", "add-feature"]),
        "greenfield reconcile prepends a single app-foundation bootstrap slice"
    );
    assert_eq!(summary["slice-count"], 2);

    // The bootstrap slice lands in the same atomic plan write, depended
    // on by the feature slice.
    let plan = Plan::load(&tmp.path().join("plan.yaml")).expect("load plan");
    let names: Vec<&str> = plan.entries.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["app-foundation", "add-feature"]);
    assert_eq!(plan.entries[1].depends_on, ["app-foundation"]);

    // Exactly one reconcile event carrying the augmented slice set.
    let events = journal_events(tmp.path());
    assert_eq!(events.len(), 1, "one plan.reconcile.completed event, got:\n{events:#?}");
    assert_eq!(events[0]["event"], "plan.reconcile.completed");
    assert_eq!(events[0]["payload"]["plan-name"], "platform-app");
    assert_eq!(events[0]["payload"]["slice-count"], 2);
    assert_eq!(
        events[0]["payload"]["slice-names"],
        serde_json::json!(["app-foundation", "add-feature"])
    );
}

#[test]
fn reconcile_incremental_bootstraps() {
    // With the `core` shell already present on disk, only the absent
    // `ios` / `android` shells get per-platform `bootstrap-<platform>`
    // slices (no `app-foundation` collapse).
    let tmp = platform_project("core,ios,android");
    let core_shell = tmp.path().join("shared/src/app.rs");
    fs::create_dir_all(core_shell.parent().unwrap()).expect("mkdir shared/src");
    fs::write(&core_shell, "fn app() {}\n").expect("write core shell");

    let summary = propose_reconcile_ok(tmp.path(), PLATFORM_RESPONSE);
    assert_eq!(
        summary["slice-names"],
        serde_json::json!(["bootstrap-ios", "bootstrap-android", "add-feature"]),
        "an existing core shell yields per-platform bootstrap slices"
    );

    let events = journal_events(tmp.path());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["event"], "plan.reconcile.completed");
    assert_eq!(
        events[0]["payload"]["slice-names"],
        serde_json::json!(["bootstrap-ios", "bootstrap-android", "add-feature"])
    );
}

// -- negative: command-mode + response read/parse gates --------------

#[test]
fn propose_mode_required() {
    let project = Project::init();
    project.seed_plan("name: demo\nslices: []\n");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "propose"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let body = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(body["error"], "plan-propose-mode-required");
}

#[test]
fn propose_response_not_found() {
    let project = Project::init();
    project.seed_plan("name: demo\nslices: []\n");
    let missing = project.root().join("absent.json");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "propose", "--from"])
        .arg(&missing)
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let body = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(body["error"], "plan-propose-response-not-found");
}

#[test]
fn propose_response_schema_rejected() {
    let project = Project::init();
    project.seed_plan("name: demo\nslices: []\n");
    seed_discovery(project.root(), &discovery_doc(&[("docs", "a")]));

    // Drop the required `kind` discriminator: the envelope matches
    // neither `oneOf` branch and is rejected by the schema gate before
    // the structural deserialise.
    let body = propose_from_stderr(
        project.root(),
        r#"{"version":1,"slices":[{"name":"a","sources":[{"source":"docs","lead":"a"}]}]}"#,
    );
    assert_eq!(body["error"], "proposal-schema");
}

// -- negative: propagated `plan-reconcile-*` codes -------------------
//
// One representative fixture: each `plan-reconcile-*` invariant is
// exercised per-code at the kernel unit layer
// (`crates/workflow/src/change/plan/core/propose/tests.rs`); the binary
// layer only locks the exit-2 stderr propagation path they all share.

#[test]
fn propose_reconcile_lead_orphan() {
    let project = Project::init();
    project.seed_plan("name: demo\nslices: []\n");
    seed_discovery(project.root(), &discovery_doc(&[("docs", "real")]));

    let body = propose_from_stderr(
        project.root(),
        r#"{"version":1,"kind":"response","slices":[{"name":"s","sources":[{"source":"docs","lead":"ghost"}]}]}"#,
    );
    assert_eq!(body["error"], "plan-reconcile-lead-orphan");
}

#[test]
fn propose_dry_run_empty_catalog() {
    // `plan-reconcile-empty-catalog` is reachable via --dry-run (no
    // surveyed leads). Under --from it is masked by lead-orphan /
    // partition, since a schema-valid response must cite at least one
    // lead against the empty catalog.
    let project = Project::init();
    project.seed_plan("name: demo\nslices: []\n");
    // Deliberately no discovery.md.

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "propose", "--dry-run"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(2));
    let body = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(body["error"], "plan-reconcile-empty-catalog");
}

// -- re-propose semantics --------------------------------------------

#[test]
fn propose_re_propose_replaces_all_slices() {
    // `--from` is a wholesale projection, not a merge: a second run on a
    // still-pending plan replaces the prior slice set entirely.
    let project = Project::init();
    project.seed_plan(PROPOSE_PLAN_N1);
    seed_discovery(project.root(), PROPOSE_DISCOVERY_N1);

    propose_from_ok(
        project.root(),
        r#"{"version":1,"kind":"response","slices":[{"name":"first","sources":[{"source":"intent","lead":"fix-typo"}]}]}"#,
    );
    let plan_after_first = Plan::load(&project.plan_path()).expect("load plan");
    assert_eq!(
        plan_after_first.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["first"]
    );

    propose_from_ok(
        project.root(),
        r#"{"version":1,"kind":"response","slices":[{"name":"second","sources":[{"source":"intent","lead":"fix-typo"}]}]}"#,
    );
    let plan_after_second = Plan::load(&project.plan_path()).expect("load plan");
    assert_eq!(
        plan_after_second.entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(),
        ["second"],
        "the second --from wholesale-replaces the first slice set"
    );
}

#[test]
fn propose_refuses_on_approved_plan() {
    // Once the operator stamps Gate 1 (`approved`), the plan is no
    // longer replaceable and `--from` aborts.
    let project = Project::init();
    project.seed_plan(PROPOSE_PLAN_N1);
    seed_discovery(project.root(), PROPOSE_DISCOVERY_N1);

    propose_from_ok(project.root(), PROPOSE_RESPONSE_N1);
    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "transition", "demo", "approved"])
        .assert()
        .success();

    let body = propose_from_stderr(project.root(), PROPOSE_RESPONSE_N1);
    assert_eq!(body["error"], "plan-reconcile-plan-not-replaceable");
}

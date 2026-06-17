//! Plan-driven loop acceptance proof.
//!
//! This is the end-to-end fixture that proves fan-in *twice* (Lead sets
//! at `survey`, Evidence at `extract`) and fan-out *once* (multiple
//! single-target slices reconciled from shared source claims), then
//! drives those slices all the way through `build` and `merge` under a
//! `depends-on` ordering and asserts the kernel-projection determinism
//! property.
//!
//! ```text
//! documentation + typescript (sources: docs, legacy)
//!   -> source survey            # fan-in #1: Lead sets (incl. docs:password-reset / legacy:reset-password mismatch)
//!   -> plan propose --dry-run   # flat lead catalog + identity-contracts->contracts@1.0.0 / identity-service->omnia@1.0.0
//!   -> plan propose --from      # agent groups leads; kernel writes single-target slices + project bindings + depends-on
//!   -> per slice: source extract -> slice synthesize -> slice build -> slice merge
//!   -> depends-on ordering: identity-contracts merges before identity-service starts
//! ```
//!
//! ## Topology choice (documented simplification)
//!
//! An alternative *same-tree registry-symlink* topology exists, where
//! two registry projects resolve into one working tree via `registry.yaml`
//! URLs materialised as symlinks. This
//! test uses the **workspace + committed `topology.lock`** projection that the
//! shipped `plan propose` tests already exercise (see
//! `tests/workflow/propose.rs::propose_*`) — it exposes the same two
//! projects to `propose` without the symlink-materialisation machinery,
//! which the deterministic proof does not need. Both slices live in one
//! `.specify/slices/` tree and merge into one baseline (`.specify/specs/`),
//! so "two single-target slices sharing one baseline tree, ordered by
//! depends-on" is proven directly. Each slice's bound target is set via
//! `slice create --target <t>` (the CLI surface that stores the bound
//! adapter on `metadata.yaml`); `slice build` resolves it from there,
//! exactly as in production.
//!
//! ## Coverage delegated to existing tests (not re-implemented here)
//!
//! The exhaustive malformed-`--from` reconcile codes
//! (`lead-coverage-orphan`, `plan-reconcile-lead-orphan`,
//! `-slice-source-collision`, `-slice-name-collision`,
//! `-depends-on-cycle`, `-project-binding-required`, `-project-orphan`,
//! `-plan-not-replaceable`) are covered over this exact identity fan-out
//! shape in `tests/workflow/`. The synthesis-kernel
//! normalize-not-reject and per-source orphan/kind-mismatch aborts are
//! covered in `tests/slice.rs::synthesize_normalizes_pre_assigned_fields`.
//! This test asserts the *composed* path and the fan-out-specific guards
//! (`plan-propose-mode-required` plus a `project-binding-required`
//! spot-check on this workspace), then the build / merge / ordering /
//! determinism behaviour no existing test covers.
//!
//! `change.md` rendering of cross-source matches is **agent-owned**: the
//! response `rationale` field is kernel-ignored (see
//! `change/plan/core/propose.rs`), so `change.md` is authored by the
//! `/spec:plan` skill, not the deterministic CLI this test drives. We
//! therefore assert the kernel-side effects of the fan-out (slice
//! bindings, depends-on, the `plan.reconcile.completed` event) rather
//! than the skill-authored `change.md`.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;
use specify_workflow::design_system::{ComponentStatus, ComponentsCatalog};
use tempfile::{TempDir, tempdir};

use crate::common::{
    Project, copy_dir, hold_plan_lock, init_workspace, omnia_schema_dir, parse_json, parse_stderr,
    parse_stdout, repo_root, sha256_hex, specify_cmd,
};

// ---------------------------------------------------------------------------
// Fixture + seed material
// ---------------------------------------------------------------------------

fn fixture_dir() -> PathBuf {
    repo_root().join("tests/fixtures/fan-in-fan-out")
}

fn fixture(rel: &str) -> String {
    let path = fixture_dir().join(rel);
    fs::read_to_string(&path).unwrap_or_else(|err| panic!("read fixture {}: {err}", path.display()))
}

/// Hub registry: the two projects bound to different target adapters
/// that the fan-out response binds against.
const REGISTRY_HUB: &str = "\
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

/// Committed plan-time topology projection the workspace reads in
/// place of materialising the remote members.
const TOPOLOGY_HUB: &str = "\
version: 1
projects:
  - name: identity-contracts
    target: contracts@1.0.0
    description: Versioned API contracts crate for the identity domain.
  - name: identity-service
    target: omnia@1.0.0
    description: Omnia identity service implementing auth and password flows.
";

/// Hub plan declaring the two surveyed sources, no slices yet.
const PLAN_HUB: &str = "\
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

const CONTRACTS_ADAPTER: &str = "\
name: contracts
version: 1.0.0
axis: target
execution: agent
briefs:
  shape: briefs/shape.md
  build: briefs/build.md
  merge: briefs/merge.md
inputs:
  - path: contracts
    required: true
description: Versioned API contracts target.
";

// ---------------------------------------------------------------------------
// Staging helpers
// ---------------------------------------------------------------------------

/// Author a minimal `execution: agent` source adapter with the two
/// briefs the survey/extract agents read.
fn stage_source_adapter(root: &Path, name: &str, description: &str) {
    let dir = root.join(format!("adapters/sources/{name}"));
    fs::create_dir_all(dir.join("briefs")).expect("mkdir source adapter briefs");
    fs::write(
        dir.join("adapter.yaml"),
        format!(
            "name: {name}\nversion: 1.0.0\naxis: source\nexecution: agent\nbriefs:\n  survey: \
             briefs/survey.md\n  extract: briefs/extract.md\ndescription: {description}\n"
        ),
    )
    .expect("write source adapter.yaml");
    fs::write(dir.join("briefs/survey.md"), "# survey brief\n").expect("write survey brief");
    fs::write(dir.join("briefs/extract.md"), "# extract brief\n").expect("write extract brief");
}

/// Author the `contracts` target adapter (declares a required
/// `contracts` build input) and copy the in-repo `omnia` target.
fn stage_target_adapters(root: &Path) {
    copy_dir(&omnia_schema_dir(), &root.join("adapters/targets/omnia"));
    let dir = root.join("adapters/targets/contracts");
    fs::create_dir_all(dir.join("briefs")).expect("mkdir contracts adapter briefs");
    fs::write(dir.join("adapter.yaml"), CONTRACTS_ADAPTER).expect("write contracts adapter.yaml");
    for op in ["shape", "build", "merge"] {
        fs::write(dir.join(format!("briefs/{op}.md")), format!("# {op} brief\n"))
            .expect("write contracts brief");
    }
}

/// Stand in for the survey agent: drop the golden lead-set into scratch
/// and run `source survey <source> --phase finalize`.
fn survey_finalize(root: &Path, source: &str, adapter: &str, lead_set: &str) {
    let scratch = root.join(format!(".specify/scratch/{adapter}/survey"));
    fs::create_dir_all(&scratch).expect("mkdir survey scratch");
    fs::write(scratch.join("leads.md"), lead_set).expect("write leads.md");
    specify_cmd()
        .current_dir(root)
        .args(["source", "survey", source, "--phase", "finalize"])
        .assert()
        .success();
}

/// Stand in for the extract agent: drop the golden Evidence into scratch
/// and run `source extract <source> <lead> --slice <slice> --phase finalize`.
fn extract_finalize(
    root: &Path, source: &str, adapter: &str, lead: &str, slice: &str, evidence: &str,
) {
    let scratch = root.join(format!(".specify/scratch/{adapter}/{slice}"));
    fs::create_dir_all(&scratch).expect("mkdir extract scratch");
    fs::write(scratch.join("evidence.yaml"), evidence).expect("write evidence.yaml");
    specify_cmd()
        .current_dir(root)
        .args(["source", "extract", source, lead, "--slice", slice, "--phase", "finalize"])
        .assert()
        .success();
}

fn journal_lines(root: &Path) -> Vec<String> {
    let path = root.join(".specify/journal.jsonl");
    fs::read_to_string(&path)
        .map(|raw| raw.lines().filter(|l| !l.is_empty()).map(str::to_string).collect())
        .unwrap_or_default()
}

fn journal_has(root: &Path, event: &str) -> bool {
    journal_lines(root).iter().any(|l| l.contains(&format!(r#""event":"{event}""#)))
}

fn read_plan(root: &Path) -> String {
    fs::read_to_string(root.join("plan.yaml")).expect("read plan.yaml")
}

// ---------------------------------------------------------------------------
// Scenario setup
// ---------------------------------------------------------------------------

/// Stand up the workspace, stage adapters + sources, and run both
/// surveys so `discovery.md` carries all four leads.
fn scenario() -> TempDir {
    let tmp = tempdir().expect("tempdir");
    let root = tmp.path();
    init_workspace(&tmp, "identity-revamp");
    fs::write(root.join("registry.yaml"), REGISTRY_HUB).expect("write registry.yaml");
    fs::write(root.join(".specify/topology.lock"), TOPOLOGY_HUB).expect("write topology.lock");
    fs::write(root.join("plan.yaml"), PLAN_HUB).expect("write plan.yaml");

    stage_source_adapter(root, "documentation", "Operator-provided written intent.");
    stage_source_adapter(root, "typescript", "Behavioural evidence from a TypeScript codebase.");
    stage_target_adapters(root);

    // Both bound source paths exist on disk for the survey/extract runs.
    for src in ["docs", "legacy"] {
        fs::create_dir_all(root.join(src)).expect("mkdir bound source dir");
        fs::write(root.join(src).join(".keep"), "").expect("seed bound source dir");
    }

    // Fan-in #1: survey both sources into one discovery.md.
    survey_finalize(root, "docs", "documentation", &fixture("leads/docs.md"));
    survey_finalize(root, "legacy", "typescript", &fixture("leads/legacy.md"));
    tmp
}

// ---------------------------------------------------------------------------
// The acceptance proof
// ---------------------------------------------------------------------------

/// The plan-time half of the proof: survey leads (with the deliberate
/// slug mismatch), the `--dry-run` request envelope, the
/// `plan-propose-mode-required` / `project-binding-required` guards, and
/// the `--from` fan-out that writes single-target slices with project
/// bindings + depends-on and emits `plan.reconcile.completed`.
fn prove_plan_time_fan_out(root: &Path) {
    // Survey produced schema-valid leads, including the deliberate
    // docs:password-reset / legacy:reset-password slug mismatch.
    let discovery = fs::read_to_string(root.join("discovery.md")).expect("read discovery.md");
    for block in [
        "### docs:identity-api",
        "### docs:password-reset",
        "### legacy:identity-api",
        "### legacy:reset-password",
    ] {
        assert!(discovery.contains(block), "discovery.md missing {block}, got:\n{discovery}");
    }

    // `propose --dry-run` returns a kind:request envelope exposing both
    // projects and one lead row per (source, lead), and writes nothing.
    let plan_before = read_plan(root);
    let dry = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "plan", "propose", "--dry-run"])
        .assert()
        .success();
    let request = parse_stdout(&dry.get_output().stdout, root);
    assert_eq!(request["kind"], "request");
    let projects = request["projects"].as_array().expect("projects array");
    assert_eq!(projects.len(), 2);
    assert_eq!(projects[0]["name"], "identity-contracts");
    assert_eq!(projects[0]["target"], "contracts@1.0.0");
    assert_eq!(projects[1]["name"], "identity-service");
    assert_eq!(projects[1]["target"], "omnia@1.0.0");
    let leads = request["leads"].as_array().expect("leads array");
    assert_eq!(leads.len(), 4, "one row per (source, lead): {leads:#?}");
    assert_eq!(read_plan(root), plan_before, "--dry-run must not touch plan.yaml");

    // Neither flag aborts mode-required.
    let no_mode = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "plan", "propose"])
        .assert()
        .failure();
    assert_eq!(no_mode.get_output().status.code(), Some(2));
    assert_eq!(
        parse_stderr(&no_mode.get_output().stderr, root)["error"],
        "plan-propose-mode-required"
    );

    // Fan-out-specific reconcile guard: with two projects offered, a slice
    // that covers its leads cleanly but omits `project` aborts
    // project-binding-required. (The full partition is satisfied so this
    // is not a partition/collision abort.) The remaining reconcile codes
    // are covered exhaustively over this shape in tests/workflow/.
    fs::write(
        root.join("bad-response.json"),
        r#"{"version":1,"kind":"response","slices":[{"name":"unbound","sources":[{"source":"docs","lead":"identity-api"},{"source":"legacy","lead":"identity-api"}]},{"name":"reset","project":"identity-service","sources":[{"source":"docs","lead":"password-reset"},{"source":"legacy","lead":"reset-password"}]}]}"#,
    )
    .expect("write bad response");
    let bound = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "plan", "propose", "--from", "bad-response.json"])
        .assert()
        .failure();
    assert_eq!(
        parse_stderr(&bound.get_output().stderr, root)["error"],
        "plan-reconcile-project-binding-required"
    );

    // `propose --from` writes single-target slices with project bindings +
    // depends-on and emits plan.reconcile.completed.
    fs::write(root.join("response.json"), fixture("propose-response.json"))
        .expect("write response.json");
    let from = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "plan", "propose", "--from", "response.json"])
        .assert()
        .success();
    let summary = parse_stdout(&from.get_output().stdout, root);
    assert_eq!(summary["slice-count"], 3);
    assert_eq!(
        summary["slice-names"],
        serde_json::json!(["identity-contracts", "identity-service", "password-reset"])
    );
    assert!(journal_has(root, "plan.reconcile.completed"), "fan-out must emit reconcile.completed");

    let plan = read_plan(root);
    assert!(plan.contains("project: identity-contracts"), "contracts slice binds its project");
    assert!(plan.contains("project: identity-service"), "service slice binds its project");
    assert!(plan.contains("depends-on:"), "service depends-on contracts");
    // The cross-source slug mismatch is matched into the third slice.
    assert!(plan.contains("name: password-reset"), "password-reset slice present");
    assert!(plan.contains("lead: reset-password"), "legacy reset-password lead carried verbatim");
}

#[test]
fn fan_in_twice_fan_out_once() {
    let tmp = scenario();
    let root = tmp.path();

    prove_plan_time_fan_out(root);

    // From here on this test *is* the driver session: hold the plan
    // lock the way the /spec:execute snippet does, so the gated verbs
    // (`plan next`, `slice merge run`) accept the writes.
    let _lock = hold_plan_lock(root);

    // --- depends-on ordering, gate 1: the driver must pick
    // identity-contracts first — never identity-service while its upstream
    // is unmerged. -------------------------------------------------------
    assert_eq!(plan_next(root)["next"], "identity-contracts");
    // A second poll while contracts is in-progress returns the active
    // entry, never advancing to the dependent.
    let active = plan_next(root);
    assert_eq!(active["reason"], "in-progress");
    assert_eq!(active["active"], "identity-contracts");

    // --- Slice-time: drive identity-contracts (bound target: contracts). -
    drive_slice_to_built(root, "identity-contracts", "contracts", Sources::DocsOnly);

    // The contracts build request carries the adapter-declared `contracts`
    // input in `additional[]`; the bound target is `contracts`.
    let contracts_request =
        fs::read_to_string(root.join(".specify/slices/identity-contracts/build/request.yaml"))
            .expect("read contracts build request");
    assert!(
        contracts_request.contains("additional:") && contracts_request.contains("- contracts"),
        "contracts request resolves the declared `contracts` input into additional[], got:\n{contracts_request}"
    );

    specify_cmd()
        .current_dir(root)
        .args(["slice", "merge", "run", "identity-contracts"])
        .assert()
        .success();
    assert!(read_plan(root).contains("status: done"), "merge stamps the contracts entry done");
    // Upstream output is now visible in the shared baseline tree — the
    // in-tree dependency identity-service reads (no cross-slice channel).
    assert!(
        root.join(".specify/specs/identity/spec.md").is_file(),
        "contracts merge writes the shared baseline before the dependent starts"
    );

    // --- depends-on ordering, gate 2: only now does the driver advance to
    // identity-service. --------------------------------------------------
    assert_eq!(plan_next(root)["next"], "identity-service");

    // --- Slice-time: drive identity-service (bound target: omnia). ------
    drive_slice_to_built(root, "identity-service", "omnia", Sources::DocsAndLegacy);

    // The omnia build request declares no extra inputs, so additional[] is
    // absent (skip_serializing_if empty); the bound target is `omnia`.
    let service_request =
        fs::read_to_string(root.join(".specify/slices/identity-service/build/request.yaml"))
            .expect("read service build request");
    assert!(
        !service_request.contains("additional"),
        "omnia declares no extra inputs, so additional[] is omitted, got:\n{service_request}"
    );

    specify_cmd()
        .current_dir(root)
        .args(["slice", "merge", "run", "identity-service"])
        .assert()
        .success();

    // --- Final plan state: both driven slices done; the cross-source
    // password-reset slice remains pending (proven at plan time only). ---
    let final_plan = read_plan(root);
    let done = final_plan.matches("status: done").count();
    assert_eq!(
        done, 2,
        "both identity-contracts and identity-service reach done, got:\n{final_plan}"
    );
    assert!(
        final_plan.contains("status: pending"),
        "password-reset stays pending, got:\n{final_plan}"
    );
}

/// Which `(source, lead)` pairs a slice extracts Evidence for.
#[derive(Clone, Copy)]
enum Sources {
    DocsOnly,
    DocsAndLegacy,
}

/// Run `plan next --format json` and return the parsed body.
fn plan_next(root: &Path) -> Value {
    let out = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "plan", "next"])
        .assert()
        .success();
    parse_json(&out.get_output().stdout)
}

/// Create a slice bound to `target`, extract its Evidence, synthesize it,
/// assert the slice-time invariants, then build it to `built`.
fn drive_slice_to_built(root: &Path, slice: &str, target: &str, sources: Sources) {
    specify_cmd()
        .current_dir(root)
        .args(["slice", "create", slice, "--target", target])
        .assert()
        .success();

    // Fan-in #2: Evidence per (slice, source).
    extract_finalize(
        root,
        "docs",
        "documentation",
        "identity-api",
        slice,
        &fixture(&format!("evidence/{slice}/docs.yaml")),
    );
    if matches!(sources, Sources::DocsAndLegacy) {
        extract_finalize(
            root,
            "legacy",
            "typescript",
            "identity-api",
            slice,
            &fixture(&format!("evidence/{slice}/legacy.yaml")),
        );
    }
    let evidence_dir = root.join(format!(".specify/slices/{slice}/evidence"));
    assert!(evidence_dir.join("docs.yaml").is_file(), "{slice} docs Evidence persisted");
    if matches!(sources, Sources::DocsAndLegacy) {
        assert!(evidence_dir.join("legacy.yaml").is_file(), "{slice} legacy Evidence persisted");
    }

    // Synthesis: project the agent response into model.yaml + artifacts.
    fs::write(root.join("synth.json"), fixture(&format!("synthesis/{slice}.json")))
        .expect("write synth response");
    let synth = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "slice", "synthesize", slice, "--from", "synth.json"])
        .assert()
        .success();
    let artifacts: Vec<String> = parse_json(&synth.get_output().stdout)["artifacts"]
        .as_array()
        .expect("artifacts array")
        .iter()
        .map(|a| a.as_str().unwrap_or_default().to_string())
        .collect();
    for expected in ["proposal.md", "specs/identity/spec.md", "design.md", "tasks.md", "model.yaml"]
    {
        assert!(artifacts.contains(&expected.to_string()), "{slice} missing {expected}");
    }

    // model.yaml carries inline provenance; `slice validate` flags no
    // staleness; `slice provenance` projects the audit view.
    let validate = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "slice", "validate", slice])
        .assert();
    assert_no_staleness(validate.get_output());
    specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "slice", "provenance", slice])
        .assert()
        .success();

    // Contracts declares a required `contracts` build input; seed the
    // slice tree so request assembly resolves it.
    if target == "contracts" {
        let contracts_dir = root.join(format!(".specify/slices/{slice}/contracts"));
        fs::create_dir_all(&contracts_dir).expect("mkdir slice contracts");
        fs::write(contracts_dir.join("openapi.yaml"), fixture("contracts-input/openapi.yaml"))
            .expect("seed contracts input");
    }

    specify_cmd()
        .current_dir(root)
        .args(["slice", "transition", slice, "refined"])
        .assert()
        .success();

    // Build, prepare phase: assemble + schema-validate + persist request.
    let prepare = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "slice", "build", slice])
        .assert()
        .success();
    let handoff = parse_json(&prepare.get_output().stdout);
    assert_eq!(handoff["slice"], slice);
    assert_eq!(handoff["target"], target);
    assert_eq!(handoff["execution"], "agent");
    assert!(journal_has(root, "target.execution.agent"), "prepare emits target.execution.agent");

    // Build, finalize phase: validate the golden report + gate `built`.
    fs::write(
        root.join(format!(".specify/slices/{slice}/build/report.yaml")),
        fixture(&format!("reports/{slice}.yaml")),
    )
    .expect("write golden build report");
    let finalize = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "slice", "build", slice, "--phase", "finalize"])
        .assert()
        .success();
    let result = parse_json(&finalize.get_output().stdout);
    assert_eq!(result["status"], "success");
    assert!(journal_has(root, "slice.build.started"));
    assert!(journal_has(root, "slice.build.succeeded"));
    let meta = fs::read_to_string(root.join(format!(".specify/slices/{slice}/metadata.yaml")))
        .expect("read slice metadata");
    assert!(meta.contains("status: built"), "finalize gates `built`, got:\n{meta}");
}

/// Assert the rendered `DiagnosticReport` on stdout carries no
/// slice-model / provenance staleness finding. Tolerates unrelated
/// adapter findings — the slice-time assertion is specifically "no
/// staleness".
fn assert_no_staleness(output: &std::process::Output) {
    let Ok(report) = serde_json::from_slice::<Value>(&output.stdout) else {
        return;
    };
    let Some(findings) = report["findings"].as_array() else {
        return;
    };
    for rule in [
        "slice-model-schema",
        "slice-spec-provenance-stale",
        "slice-model-target-drift",
        "slice-model-source-orphan",
        "slice-model-cross-ref-orphan",
        "slice-model-claim-kind-mismatch",
        "slice-model-id-grammar",
    ] {
        assert!(
            findings.iter().all(|f| f["rule-id"] != rule),
            "staleness rule {rule} must not fire on a freshly synthesized slice: {findings:#?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Non-blocking determinism property
// ---------------------------------------------------------------------------

/// Re-running kernel projection twice over a golden synthesis response
/// yields byte-identical kernel-owned `model.yaml` fields, and the
/// projection is target-independent: the same Evidence + response
/// projected for two slices bound to *different* targets yields
/// identical kernel-owned requirements, and the model carries no
/// target/adapter field.
#[test]
fn kernel_projection_deterministic() {
    let project = Project::init().with_schemas();
    let root = project.root();

    // Two slices bound to different targets; `slice build` resolves the
    // target from `metadata.yaml`, but the synthesis kernel never sees
    // it — that target-independence is what this test pins.
    project.seed_plan(
        "\
name: determinism
sources:
  docs:
    adapter: documentation
    path: ./docs
  legacy:
    adapter: typescript
    path: ./legacy
slices:
  - name: bound-contracts
    project: identity-contracts
    status: pending
    sources:
      - { source: docs, lead: identity-api }
  - name: bound-omnia
    project: identity-service
    status: pending
    sources:
      - { source: docs, lead: identity-api }
",
    );

    let evidence = fixture("evidence/identity-contracts/docs.yaml");
    let response = fixture("synthesis/identity-contracts.json");
    fs::write(root.join("synth.json"), &response).expect("write synth response");

    let mut requirements: Vec<Value> = Vec::new();
    for (slice, target) in [("bound-contracts", "contracts"), ("bound-omnia", "omnia")] {
        specify_cmd()
            .current_dir(root)
            .args(["slice", "create", slice, "--target", target])
            .assert()
            .success();
        let evidence_dir = project.slices_dir().join(format!("{slice}/evidence"));
        fs::create_dir_all(&evidence_dir).expect("mkdir evidence");
        fs::write(evidence_dir.join("docs.yaml"), &evidence).expect("write evidence");

        specify_cmd()
            .current_dir(root)
            .args(["slice", "synthesize", slice, "--from", "synth.json"])
            .assert()
            .success();

        let show = specify_cmd()
            .current_dir(root)
            .args(["--format", "json", "slice", "model", "show", slice])
            .assert()
            .success();
        let model = parse_json(&show.get_output().stdout);
        assert!(model.get("target").is_none(), "kernel model carries no target field");
        assert!(model.get("adapter").is_none(), "kernel model carries no adapter field");
        requirements.push(model["requirements"].clone());
    }

    // Target-independence: the kernel-owned requirements are identical
    // across the two differently-targeted slices.
    assert_eq!(
        requirements[0], requirements[1],
        "kernel-owned requirements must be target-independent"
    );

    // Byte-identical re-projection: a second `--from` over the same golden
    // response reproduces the same `model.yaml` exactly.
    let model_path = project.slices_dir().join("bound-contracts/model.yaml");
    let first = fs::read_to_string(&model_path).expect("first model.yaml");
    specify_cmd()
        .current_dir(root)
        .args(["slice", "synthesize", "bound-contracts", "--from", "synth.json"])
        .assert()
        .success();
    let second = fs::read_to_string(&model_path).expect("second model.yaml");
    assert_eq!(first, second, "re-running projection must be byte-identical");
}

// ---------------------------------------------------------------------------
// Composition accumulation + the baseline-overwrite gate
// ---------------------------------------------------------------------------
//
// The merge-kernel accumulation + gate assertions live in
// `crates/workflow/tests/merge_slice.rs`. These two tests are
// the integration layer: they drive whole `specify slice merge run`
// invocations across several slices and assert (1) the merged baseline
// at `.specify/specs/composition.yaml` grows monotonically as
// screen-introducing slices accumulate via `delta.added`, and (2) the
// `composition-baseline-overwrite-blocked` gate fires (and is overridable
// with `--allow-composition-replace`) in a realistic multi-slice run.

/// Read the merged composition baseline's `screens` map. An absent
/// baseline or a baseline without a `screens` mapping yields an empty
/// map. Parsed with `serde_saphyr` into a `serde_json::Value`, the same
/// path the merge engine itself uses.
fn composition_screens(root: &Path) -> serde_json::Map<String, Value> {
    let path = root.join(".specify/specs/composition.yaml");
    let Ok(text) = fs::read_to_string(&path) else {
        return serde_json::Map::new();
    };
    let doc: Value = serde_saphyr::from_str(&text)
        .unwrap_or_else(|err| panic!("parse {}: {err}", path.display()));
    doc.get("screens").and_then(Value::as_object).cloned().unwrap_or_default()
}

/// A one-screen `delta: { added }` composition — the accumulating shape a
/// non-bootstrap screen-introducing slice emits.
fn delta_added(screen: &str, name: &str) -> String {
    format!(
        "version: 1\ndelta:\n  added:\n    {screen}:\n      name: {name}\n  modified: {{}}\n  \
         removed: {{}}\n"
    )
}

/// A whole-document `screens:` composition — the replacement shape the
/// overwrite gate guards against once a non-empty baseline exists.
fn whole_document(screen: &str, name: &str) -> String {
    format!("version: 1\nscreens:\n  {screen}:\n    name: {name}\n")
}

/// Create `name`, stage its top-level `composition.yaml`, and drive it to
/// `built` through the real lifecycle verbs (`slice create` →
/// `slice transition refined` → `slice transition built`) so
/// `slice merge run` accepts it. Composition-only by design: the
/// spec-delta accumulation path is covered elsewhere, so these tests
/// isolate the composition baseline behaviour.
fn stage_built_composition_slice(project: &Project, name: &str, composition: &str) {
    specify_cmd().current_dir(project.root()).args(["slice", "create", name]).assert().success();
    fs::write(project.slices_dir().join(name).join("composition.yaml"), composition)
        .expect("write slice composition");
    for target in ["refined", "built"] {
        specify_cmd()
            .current_dir(project.root())
            .args(["slice", "transition", name, target])
            .assert()
            .success();
    }
}

/// Three screen-introducing slices, each contributing one `delta.added`
/// screen, merged in sequence: the baseline `screens` map must grow
/// 1 → 2 → 3 with no prior screen lost — the data-loss regression
/// accumulation closes.
#[test]
fn composition_accumulates_across_slices() {
    let project = Project::init().with_schemas();
    let root = project.root();

    let slices = [
        ("intro-home", "home", "Home"),
        ("intro-settings", "settings", "Settings"),
        ("intro-profile", "profile", "Profile"),
    ];

    let mut accumulated: Vec<&str> = Vec::new();
    for (index, (slice, screen, name)) in slices.iter().enumerate() {
        stage_built_composition_slice(&project, slice, &delta_added(screen, name));
        specify_cmd().current_dir(root).args(["slice", "merge", "run", slice]).assert().success();

        accumulated.push(screen);
        let screens = composition_screens(root);
        assert_eq!(
            screens.len(),
            index + 1,
            "baseline must hold {} screen(s) after merging {slice}, got {screens:?}",
            index + 1
        );
        for slug in &accumulated {
            assert!(
                screens.contains_key(*slug),
                "screen `{slug}` must survive in the accumulated baseline, got {screens:?}"
            );
        }
    }
}

/// A whole-document (`screens:`) slice composition over a non-empty
/// baseline aborts `slice merge run` with
/// `composition-baseline-overwrite-blocked`; the gate is a precondition
/// (the baseline is untouched and the slice stays `built`), and
/// `--allow-composition-replace` authorises the full replacement.
#[test]
fn composition_overwrite_gate_blocks() {
    let project = Project::init().with_schemas();
    let root = project.root();

    // Establish a non-empty baseline via an accumulating first slice.
    stage_built_composition_slice(&project, "intro-home", &delta_added("home", "Home"));
    specify_cmd()
        .current_dir(root)
        .args(["slice", "merge", "run", "intro-home"])
        .assert()
        .success();
    assert_eq!(composition_screens(root).len(), 1);

    // A whole-document slice composition is blocked over the non-empty
    // baseline.
    stage_built_composition_slice(
        &project,
        "rewrite-all",
        &whole_document("dashboard", "Dashboard"),
    );
    let blocked = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "slice", "merge", "run", "rewrite-all"])
        .assert()
        .failure();
    assert_eq!(
        parse_stderr(&blocked.get_output().stderr, root)["error"],
        "composition-baseline-overwrite-blocked"
    );

    // Precondition semantics: nothing moved — the baseline is intact and
    // `rewrite-all` is still `built`, so the override can re-run it.
    let preserved = composition_screens(root);
    assert_eq!(preserved.len(), 1, "blocked merge must not touch the baseline");
    assert!(preserved.contains_key("home"), "the prior screen must survive a blocked merge");

    // The narrow override authorises the whole-document replacement.
    specify_cmd()
        .current_dir(root)
        .args(["slice", "merge", "run", "rewrite-all", "--allow-composition-replace"])
        .assert()
        .success();
    let replaced = composition_screens(root);
    assert_eq!(replaced.len(), 1);
    assert!(
        replaced.contains_key("dashboard"),
        "the override replaces the baseline with the slice document, got {replaced:?}"
    );
    assert!(
        !replaced.contains_key("home"),
        "whole-document replacement drops the prior screen, got {replaced:?}"
    );
}

// ---------------------------------------------------------------------------
// Composition + component-inference acceptance capstone
// ---------------------------------------------------------------------------
//
// The integration capstone exercises the full composition-accumulation +
// component-inference loop end-to-end and locks the headline behaviours:
//
//   1. Composition accumulates monotonically across screen-introducing
//      slices (Part A): a whole-document first slice establishes the
//      baseline, then `delta.added` slices grow it without loss.
//   2. `specify catalog infer` clusters the repeated `group` structure
//      across the accumulated baseline (Part B): after the second screen
//      lands, the report surfaces exactly one cluster at `occurrences: 2`,
//      and `bind` records the *skill-supplied* slug at `status: confirmed`.
//      Because naming is a skill judgement the CLI cannot perform, the
//      test stands in for the agent: it takes the reported fingerprint and
//      writes a fixed `{ <fingerprint>: shared-nav }` bindings map.
//   3. Retroactive cross-slice factoring: the build that discovers
//      the component emits directive-only `delta.modified` entries that
//      attach the `component:` directive to prior-slice screens, and the
//      simulated writer brief drops the shared component module into the
//      shell tree.
//   4. A documentation-only slice surfaces the
//      `composition-unexpected-for-non-ui-slice` warning at finalize and
//      is stopped by the `composition-baseline-overwrite-blocked` gate
//      at merge — the motivating data-loss scenario, now closed twice
//      over.
//
// Per the plan, code-generation assertions stay focused on the
// composition / catalog artifacts and the *presence* of the shared
// component module path, not full compilable shells (those live in the
// manual `acceptance/` packs). Crucially, the test asserts the
// *mechanism* — clustering, the report shape, and the binding guards —
// and never that a specific English name like `tab-bar` emerges; the slug
// is the test's own stand-in for the skill's choice.
//
// The whole scenario dispatches the real `vectis infer` tool, so it is
// skipped when the WASM artifact is absent (build it with
// `cargo make vectis-wasm`).

/// Prebuilt `vectis` WASI artifact the `catalog infer` report phase
/// dispatches. Built by `cargo make vectis-wasm`; absent on a bare
/// `cargo nextest` without that dependency.
fn vectis_wasm() -> PathBuf {
    repo_root().join("target/vectis-wasi-tools/release/vectis.wasm")
}

/// The shared bottom-navigation `footer` group repeated across every UI
/// screen in the capstone. Structurally identical instances cluster to
/// one component under the default `--min-occurrences 2`.
const SHARED_FOOTER: &str = "    footer:
      - group:
          items:
            - icon-button: { bind: home, event: Navigate(Home) }
            - icon-button: { bind: search, event: Navigate(Search) }
";

/// The same `footer` group with the factored `component: shared-nav`
/// directive attached (B7 directive-only modification). The skeleton is
/// byte-identical to [`SHARED_FOOTER`]; only the directive differs.
const FACTORED_FOOTER: &str = "    footer:
      - group:
          component: shared-nav
          items:
            - icon-button: { bind: home, event: Navigate(Home) }
            - icon-button: { bind: search, event: Navigate(Search) }
";

/// Declare the `vectis` WASI tool as a project-scoped tool in
/// `.specify/project.yaml` (read access to `.specify`), so
/// `specify catalog infer --phase report` resolves and dispatches it
/// regardless of where the bound target adapter resolves from. Tools are
/// a top-level `project.yaml` key, so the block is appended verbatim.
fn declare_vectis_tool(root: &Path, wasm: &Path) {
    let source = format!("file://{}", wasm.display());
    let sha256 = sha256_hex(wasm);
    let project_yaml = root.join(".specify/project.yaml");
    let mut config = fs::read_to_string(&project_yaml).expect("read project.yaml");
    let block = [
        "tools:",
        "  - name: vectis",
        "    version: 0.4.0",
        &format!("    source: \"{source}\""),
        &format!("    sha256: \"{sha256}\""),
        "    permissions:",
        "      read:",
        "        - \"$PROJECT_DIR/.specify\"",
        "      write: []",
        "",
    ]
    .join("\n");
    config.push_str(&block);
    fs::write(&project_yaml, config).expect("write project.yaml with vectis tool");
}

/// A minimal, schema-valid build report carrying the A4 `ui-surface`
/// signal. `target` is the bound omnia adapter; `screens` is the
/// brief-authored count of screen-bearing requirements this slice
/// introduces or modifies.
fn build_report(name: &str, ui_screens: u32) -> String {
    format!(
        "version: 1\nslice: {name}\ntarget: omnia\nstatus: success\nfindings: []\nui-surface:\n  \
         screens: {ui_screens}\n"
    )
}

/// Create a slice bound to omnia, stage its top-level `composition.yaml`
/// and a build report carrying `ui-surface.screens`, drive it to
/// `refined`, then run `slice build --phase finalize` (which gates the
/// `Refined → Built` transition and runs the A4 coherence checks).
/// Returns the parsed `BuildResult` so callers can assert its warnings.
fn build_ui_slice(root: &Path, name: &str, composition: &str, ui_screens: u32) -> Value {
    specify_cmd()
        .current_dir(root)
        .args(["slice", "create", name, "--target", "omnia"])
        .assert()
        .success();
    let slice_dir = root.join(".specify/slices").join(name);
    fs::write(slice_dir.join("composition.yaml"), composition).expect("write slice composition");
    let build_dir = slice_dir.join("build");
    fs::create_dir_all(&build_dir).expect("mkdir slice build dir");
    fs::write(build_dir.join("report.yaml"), build_report(name, ui_screens))
        .expect("write build report");

    specify_cmd()
        .current_dir(root)
        .args(["slice", "transition", name, "refined"])
        .assert()
        .success();

    let finalize = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "slice", "build", name, "--phase", "finalize"])
        .assert()
        .success();
    parse_json(&finalize.get_output().stdout)
}

/// Whether a finalize `BuildResult` carries a warning with `rule-id`.
fn has_warning(result: &Value, rule_id: &str) -> bool {
    result["warnings"].as_array().is_some_and(|w| w.iter().any(|d| d["rule-id"] == rule_id))
}

/// The `component:` directive on `<screen>`'s first `footer` group in the
/// merged baseline, if any.
fn screen_footer_component(root: &Path, screen: &str) -> Option<String> {
    composition_screens(root)
        .get(screen)?
        .get("footer")?
        .get(0)?
        .get("group")?
        .get("component")?
        .as_str()
        .map(str::to_string)
}

/// Run `specify catalog infer --phase report` with the vectis tool cache
/// pointed at `cache`, returning the parsed name-free cluster report.
fn infer_report(root: &Path, cache: &Path) -> Value {
    let out = specify_cmd()
        .current_dir(root)
        .env("SPECIFY_EXTENSIONS_CACHE", cache)
        .args(["--format", "json", "catalog", "infer", "--phase", "report"])
        .assert()
        .success();
    parse_json(&out.get_output().stdout)
}

/// The end-to-end composition acceptance proof: composition accumulation,
/// agent-simulated component inference + binding, retroactive cross-slice
/// factoring, and the non-UI-slice safety nets, all in one loop.
#[test]
fn composition_inference_capstone() {
    let wasm = vectis_wasm();
    if !wasm.is_file() {
        eprintln!(
            "skipping composition_inference_capstone: vectis WASM not found at {}; run \
             `cargo make vectis-wasm`",
            wasm.display()
        );
        return;
    }

    let project = Project::init().with_schemas();
    let root = project.root();
    declare_vectis_tool(root, &wasm);
    let cache = root.join(".tools-cache");
    fs::create_dir_all(&cache).expect("mkdir tools cache");

    accumulate_two_screens(root);
    infer_and_bind_shared_nav(root, &cache);
    factor_third_slice(root);
    assert_doc_only_safety_nets(root);
}

/// Slices 1 + 2 (`home`, `search`), each carrying the same footer group:
/// the first is a whole-document baseline, the second a `delta.added`
/// that accumulates. Asserts the baseline grows 1 → 2 with no loss, and
/// that a faithful UI slice raises no A4 warning at finalize.
fn accumulate_two_screens(root: &Path) {
    let home_doc = format!("version: 1\nscreens:\n  home:\n    name: Home\n{SHARED_FOOTER}");
    let result = build_ui_slice(root, "intro-home", &home_doc, 1);
    assert!(
        !has_warning(&result, "composition-unexpected-for-non-ui-slice")
            && !has_warning(&result, "composition-empty-for-ui-slice"),
        "a faithful UI slice is self-consistent at finalize: {result}"
    );
    merge_slice(root, "intro-home");
    assert_eq!(composition_screens(root).len(), 1, "baseline holds home after the first merge");

    let search_doc = format!(
        "version: 1\ndelta:\n  added:\n    search:\n      name: Search\n{}\n  modified: {{}}\n  \
         removed: {{}}\n",
        indent_block(SHARED_FOOTER)
    );
    build_ui_slice(root, "intro-search", &search_doc, 1);
    merge_slice(root, "intro-search");
    let screens = composition_screens(root);
    assert_eq!(screens.len(), 2, "baseline accumulates to home + search");
    assert!(screens.contains_key("home") && screens.contains_key("search"));
}

/// Inference (B2/B3): the report clusters the two identical footer groups
/// into one name-free cluster at `occurrences: 2`; the test stands in for
/// the skill, binding the reported fingerprint to a fixed slug. Asserts
/// the catalog records it `confirmed` and a re-run echoes the bound slug
/// (run-to-run stability).
fn infer_and_bind_shared_nav(root: &Path, cache: &Path) {
    let report = infer_report(root, cache);
    let clusters = report["clusters"].as_array().expect("clusters array");
    assert_eq!(clusters.len(), 1, "exactly one above-threshold cluster: {report}");
    assert_eq!(clusters[0]["occurrences"], 2);
    assert_eq!(clusters[0]["bound-slug"], Value::Null, "the CLI proposes no name");
    let fingerprint = clusters[0]["fingerprint"].as_str().expect("cluster fingerprint").to_string();

    let bindings_path = root.join(".specify/bindings.yaml");
    fs::write(&bindings_path, format!("bindings:\n  {fingerprint}: shared-nav\n"))
        .expect("write bindings");
    specify_cmd()
        .current_dir(root)
        .env("SPECIFY_EXTENSIONS_CACHE", cache)
        .args(["catalog", "infer", "--phase", "bind"])
        .arg("--bindings")
        .arg(&bindings_path)
        .assert()
        .success();
    let catalog = ComponentsCatalog::load(root).expect("catalog loads").expect("catalog written");
    assert_eq!(
        catalog.status_of("shared-nav"),
        Some(ComponentStatus::Confirmed),
        "bind records the skill-supplied slug as confirmed"
    );

    let echoed = infer_report(root, cache);
    assert_eq!(echoed["clusters"][0]["bound-slug"], "shared-nav", "report echoes the bound slug");
}

/// Slice 3 (`profile`): the discovering build introduces a new screen
/// carrying the factored component AND retroactively attaches the
/// `component:` directive to the two prior-slice screens it did not
/// author (directive-only `delta.modified`, B7). Asserts monotonic
/// accumulation to three screens, the directive on all three, and the
/// presence of the simulated shared component module path.
fn factor_third_slice(root: &Path) {
    let profile_doc = format!(
        "version: 1\ndelta:\n  added:\n    profile:\n      name: Profile\n{factored}\n  \
         modified:\n    home:\n      name: Home\n{factored}\n    search:\n      name: Search\n\
         {factored}\n  removed: {{}}\n",
        factored = indent_block(FACTORED_FOOTER)
    );
    build_ui_slice(root, "intro-profile", &profile_doc, 3);

    // Simulate the writer sub-brief dropping the shared component module
    // into the live shell tree (B7 code side); merge leaves it as residue.
    let module_path = root.join("shared/src/components/shared-nav.rs");
    fs::create_dir_all(module_path.parent().unwrap()).expect("mkdir shared components");
    fs::write(&module_path, "// shared-nav component (retroactive cross-slice factoring)\n")
        .expect("write shared component module");
    merge_slice(root, "intro-profile");

    let screens = composition_screens(root);
    assert_eq!(screens.len(), 3, "baseline accumulates to home + search + profile: {screens:?}");
    for screen in ["home", "search", "profile"] {
        assert_eq!(
            screen_footer_component(root, screen).as_deref(),
            Some("shared-nav"),
            "screen `{screen}` carries the factored component directive"
        );
    }
    assert!(module_path.is_file(), "the shared component module path is present (B7 code side)");
}

/// The documentation-only slice: it reports no UI surface
/// (`ui-surface.screens: 0`) yet emits a non-empty whole-document
/// composition — the motivating self-contradiction. Asserts A4 warns at
/// finalize and A3 stops the merge from wiping the accumulated baseline.
fn assert_doc_only_safety_nets(root: &Path) {
    let doc_only_doc = "version: 1\nscreens:\n  platform-notes:\n    name: Platform Notes\n";
    let result = build_ui_slice(root, "platform-requirements", doc_only_doc, 0);
    assert!(
        has_warning(&result, "composition-unexpected-for-non-ui-slice"),
        "A4 surfaces the non-UI-slice composition mismatch at finalize: {result}"
    );

    let blocked = specify_cmd()
        .current_dir(root)
        .args(["--format", "json", "slice", "merge", "run", "platform-requirements"])
        .assert()
        .failure();
    assert_eq!(
        parse_stderr(&blocked.get_output().stderr, root)["error"],
        "composition-baseline-overwrite-blocked",
        "A3 stops the whole-document doc-only slice from wiping the baseline"
    );

    let final_screens = composition_screens(root);
    assert_eq!(final_screens.len(), 3, "the baseline survives the blocked doc-only merge");
    assert!(!final_screens.contains_key("platform-notes"), "the doc-only screen never landed");
}

/// Run `specify slice merge run <name>`, asserting success.
fn merge_slice(root: &Path, name: &str) {
    specify_cmd().current_dir(root).args(["slice", "merge", "run", name]).assert().success();
}

/// Re-indent a top-level (4-space) `footer:` block by two more spaces so
/// it nests correctly under a `delta.added.<screen>` / `delta.modified.<screen>`
/// entry (which sits two levels deeper than a top-level `screens.<screen>`).
fn indent_block(block: &str) -> String {
    block
        .lines()
        .map(|line| if line.is_empty() { String::new() } else { format!("  {line}") })
        .collect::<Vec<_>>()
        .join("\n")
}

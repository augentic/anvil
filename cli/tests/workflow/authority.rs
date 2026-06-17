//! `plan {create,add,amend} --authority-override` CLI tests.

use crate::support::*;

// -- plan {create,add,amend} --authority-override (per-slice authority override) --------

const AUTHORITY_OVERRIDE_PLAN: &str = "\
name: identity-revamp
sources:
  legacy:
    adapter: typescript
    path: ./legacy-monolith
  runtime:
    adapter: captures
    path: ./captures/replays
slices:
  - name: identity-user-registration
    project: default
    status: pending
    sources:
      - source: legacy
        lead: user-registration
      - source: runtime
        lead: user-registration
";

fn read_journal_lines(project: &Project) -> Vec<String> {
    let path = project.root().join(".specify").join("journal.jsonl");
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(&path)
        .expect("read journal")
        .lines()
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect()
}

#[test]
fn amend_authority_override_round_trips() {
    // per-slice authority override happy path: set an override via `amend`, re-read
    // `plan.yaml` and confirm the field landed under the named
    // slice; `slice validate` accepts it because `runtime` is in
    // the slice's `sources[]`.
    let project = Project::init();
    project.seed_plan(AUTHORITY_OVERRIDE_PLAN);

    specify_cmd()
        .current_dir(project.root())
        .args([
            "plan",
            "amend",
            "identity-user-registration",
            "--authority-override",
            "identity-user-registration",
            "requirement=runtime",
        ])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        saved.contains("authority-override:"),
        "plan.yaml must contain authority-override block, got:\n{saved}"
    );
    assert!(
        saved.contains("requirement: runtime"),
        "plan.yaml must record requirement: runtime, got:\n{saved}"
    );

    // Plan-level validate passes — orphan check only fires for bad keys.
    specify_cmd().current_dir(project.root()).args(["plan", "validate"]).assert().success();

    // Journal carries exactly one PlanAmendAuthorityOverride event.
    let lines = read_journal_lines(&project);
    assert_eq!(lines.len(), 1, "expected one journal event, got:\n{lines:?}");
    let line = &lines[0];
    assert!(line.contains(r#""event":"plan.amend.authority-override""#));
    assert!(line.contains(r#""action":"set""#));
    assert!(line.contains(r#""claim-kind":"requirement""#));
    assert!(line.contains(r#""source":"runtime""#));
    assert!(line.contains(r#""slice-name":"identity-user-registration""#));
}

#[test]
fn plan_amend_override_orphan_refused() {
    // per-slice authority override gate: refuse the `specify plan amend` write when
    // the authority-override value names a source key not present
    // in the slice's `sources[]` list (`phantom`). The orphan
    // check runs in `Plan::validate` (folded in by Change 2.3),
    // which `mutate_authority_overrides` re-runs after the
    // override mutations to catch the case where a brand-new
    // entry would introduce drift.
    let project = Project::init();
    project.seed_plan(AUTHORITY_OVERRIDE_PLAN);
    let before = fs::read_to_string(project.plan_path()).expect("read plan");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args([
            "--format",
            "json",
            "plan",
            "amend",
            "identity-user-registration",
            "--authority-override",
            "identity-user-registration",
            "requirement=phantom",
        ])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(code, 2, "orphan source must exit 2 (validation_failed)");
    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "slice-authority-override-orphan-source");

    let after = fs::read_to_string(project.plan_path()).expect("read plan");
    assert_eq!(before, after, "plan.yaml must not change on the refused write");
    assert!(
        read_journal_lines(&project).is_empty(),
        "journal must stay empty on the refused write"
    );
}

#[test]
fn slice_validate_authority_override_orphan() {
    // per-slice authority override — `specify slice validate` is the per-slice gate
    // that mirrors the plan-level check; it runs before refine
    // synthesises any artifacts so a bad override is caught
    // before downstream writes. Hand-edit `plan.yaml` to seed an
    // orphan entry (the only legal path is via the CLI, which
    // refuses, so we splice the file to exercise the gate without
    // bypassing the JSON-schema enforcement).
    let project = Project::init();
    project.seed_plan(AUTHORITY_OVERRIDE_PLAN);
    let original = fs::read_to_string(project.plan_path()).expect("read plan");
    // Splice the orphan override into the first slice. Anchor on
    // the `status: pending` line so the YAML structure stays
    // wellformed regardless of source-binding ordering.
    let needle = "    status: pending\n    sources:";
    let replacement =
        "    status: pending\n    authority-override:\n      requirement: phantom\n    sources:";
    let patched = original.replacen(needle, replacement, 1);
    assert_ne!(patched, original, "splice precondition: needle present in plan.yaml");
    fs::write(project.plan_path(), patched.as_bytes()).expect("write patched plan");

    // Create the slice dir so `slice validate` runs to the gate
    // (other artifacts absent → no spec/evidence findings).
    let slices_dir =
        project.root().join(".specify").join("slices").join("identity-user-registration");
    fs::create_dir_all(&slices_dir).expect("mkdir slice");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "slice", "validate", "identity-user-registration"])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(code, 2, "slice validate orphan must exit 2 (validation_failed)");
    // `slice validate` renders the DiagnosticReport on stdout and fails
    // payload-free on stderr; the orphan finding lives on the report.
    let report = parse_stdout(&assert.get_output().stdout, project.root());
    let findings = report["findings"].as_array().expect("findings array");
    assert!(
        findings.iter().any(|r| r["rule-id"] == "slice-authority-override-orphan-source"),
        "expected orphan finding from slice validate: {findings:#?}"
    );
}

#[test]
fn amend_clear_override_removes_one() {
    // per-slice authority override: `--clear-authority-override <slice> <kind>` peels
    // off a single entry; the rest of the map survives. Journal
    // records the Clear without any spurious Set events for the
    // surviving entries.
    let project = Project::init();
    project.seed_plan(AUTHORITY_OVERRIDE_PLAN);

    specify_cmd()
        .current_dir(project.root())
        .args([
            "plan",
            "amend",
            "identity-user-registration",
            "--authority-override",
            "identity-user-registration",
            "requirement=runtime",
            "--authority-override",
            "identity-user-registration",
            "criterion=legacy",
        ])
        .assert()
        .success();

    // Wipe the journal so we observe only the second amend's events.
    fs::write(project.root().join(".specify").join("journal.jsonl"), "").expect("clear journal");

    specify_cmd()
        .current_dir(project.root())
        .args([
            "plan",
            "amend",
            "identity-user-registration",
            "--clear-authority-override",
            "identity-user-registration",
            "requirement",
        ])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        !saved.contains("requirement: runtime"),
        "requirement entry must be cleared, got:\n{saved}"
    );
    assert!(
        saved.contains("criterion: legacy"),
        "criterion entry must survive the targeted clear, got:\n{saved}"
    );

    let lines = read_journal_lines(&project);
    assert_eq!(lines.len(), 1, "expected one Clear event, got:\n{lines:?}");
    let line = &lines[0];
    assert!(line.contains(r#""action":"clear""#));
    assert!(line.contains(r#""claim-kind":"requirement""#));
}

#[test]
fn plan_amend_clear_overrides_wipes_map() {
    // per-slice authority override: `--clear-authority-overrides <slice>` wipes the
    // entire `authority-override` map for that slice and emits one
    // Clear event per kind that was present before the wipe.
    let project = Project::init();
    project.seed_plan(AUTHORITY_OVERRIDE_PLAN);

    specify_cmd()
        .current_dir(project.root())
        .args([
            "plan",
            "amend",
            "identity-user-registration",
            "--authority-override",
            "identity-user-registration",
            "requirement=runtime",
            "--authority-override",
            "identity-user-registration",
            "criterion=legacy",
        ])
        .assert()
        .success();
    fs::write(project.root().join(".specify").join("journal.jsonl"), "").expect("clear journal");

    specify_cmd()
        .current_dir(project.root())
        .args([
            "plan",
            "amend",
            "identity-user-registration",
            "--clear-authority-overrides",
            "identity-user-registration",
        ])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        !saved.contains("authority-override:"),
        "authority-override map must elide once empty, got:\n{saved}"
    );

    let lines = read_journal_lines(&project);
    assert_eq!(lines.len(), 2, "expected two per-kind Clear events, got:\n{lines:?}");
    let combined = lines.join("\n");
    assert!(combined.contains(r#""claim-kind":"requirement""#));
    assert!(combined.contains(r#""claim-kind":"criterion""#));
    assert!(
        lines.iter().all(|l| l.contains(r#""action":"clear""#)),
        "every emitted event must carry action: clear, got:\n{combined}"
    );
}

#[test]
fn amend_authority_override_set_then_clear() {
    // per-slice authority override deterministic-order rule: a same-invocation set +
    // clear pair on the same `(slice, kind)` resolves to the
    // cleared state; the journal records the Clear (not the Set).
    let project = Project::init();
    project.seed_plan(AUTHORITY_OVERRIDE_PLAN);

    specify_cmd()
        .current_dir(project.root())
        .args([
            "plan",
            "amend",
            "identity-user-registration",
            "--authority-override",
            "identity-user-registration",
            "requirement=runtime",
            "--clear-authority-override",
            "identity-user-registration",
            "requirement",
        ])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        !saved.contains("requirement: runtime"),
        "set+clear on same kind must resolve to cleared, got:\n{saved}"
    );
    let lines = read_journal_lines(&project);
    assert_eq!(lines.len(), 1, "expected one Clear event (set was elided), got:\n{lines:?}");
    assert!(
        lines[0].contains(r#""action":"clear""#),
        "the surviving event must be a clear, got:\n{}",
        lines[0]
    );
}

#[test]
fn add_authority_override_seeds_map() {
    // per-slice authority override add path: `plan add --authority-override
    // <kind>=<key>` pre-seeds the override map at create time. Each
    // entry fires one PlanAmendAuthorityOverride / `set` event.
    let project = Project::init();
    project.seed_plan(
        "name: identity-revamp\n\
        sources:\n\
        \x20\x20legacy:\n\
        \x20\x20\x20\x20adapter: typescript\n\
        \x20\x20\x20\x20path: ./legacy\n\
        \x20\x20runtime:\n\
        \x20\x20\x20\x20adapter: captures\n\
        \x20\x20\x20\x20path: ./captures/replays\n\
        slices: []\n",
    );

    specify_cmd()
        .current_dir(project.root())
        .args([
            "plan",
            "add",
            "identity-user-registration",
            "--sources",
            "legacy=user-registration",
            "--sources",
            "runtime=user-registration",
            "--authority-override",
            "requirement=runtime",
            "--authority-override",
            "criterion=legacy",
        ])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(saved.contains("authority-override:"));
    assert!(saved.contains("requirement: runtime"));
    assert!(saved.contains("criterion: legacy"));

    let lines = read_journal_lines(&project);
    assert_eq!(lines.len(), 2, "one event per seeded kind, got:\n{lines:?}");
    for line in &lines {
        assert!(line.contains(r#""action":"set""#));
        assert!(line.contains(r#""slice-name":"identity-user-registration""#));
    }
}

#[test]
fn amend_override_unknown_slice_refused() {
    // per-slice authority override: unknown `--authority-override <slice>` must
    // refuse at exit 2 before any plan.yaml write happens.
    let project = Project::init();
    project.seed_plan(AUTHORITY_OVERRIDE_PLAN);
    let before = fs::read_to_string(project.plan_path()).expect("read plan");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args([
            "--format",
            "json",
            "plan",
            "amend",
            "identity-user-registration",
            "--authority-override",
            "ghost-slice",
            "requirement=runtime",
        ])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(code, 2, "unknown slice must exit 2 (validation_failed)");

    let after = fs::read_to_string(project.plan_path()).expect("read plan");
    assert_eq!(before, after, "plan.yaml must be unchanged on refusal");
    assert!(read_journal_lines(&project).is_empty(), "no journal events on the refused write");
}

#[test]
fn plan_amend_override_bad_kind_refused() {
    // per-slice authority override: `<kind>` is validated against the closed
    // ClaimKind enum at the CLI boundary — clap surfaces a usage
    // diagnostic (exit 2) before any plan mutation runs.
    let project = Project::init();
    project.seed_plan(AUTHORITY_OVERRIDE_PLAN);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args([
            "plan",
            "amend",
            "identity-user-registration",
            "--authority-override",
            "identity-user-registration",
            "bogus-kind=runtime",
        ])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(code, 2, "invalid kind must exit 2");
    // The kind enum is enforced inside our argument parser (not by
    // clap's value_parser), so the error surfaces as a plain
    // `Error::Argument` whose stderr is human text rather than
    // JSON. We assert the exit code and the human message body.
    let stderr_str = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr_str.contains("bogus-kind"),
        "expected the bad kind name to appear in stderr, got:\n{stderr_str}"
    );
}

// ===================================================================
// `specify plan propose` — lead reconciliation
// (end-to-end coverage of the shipped command surface).
//
// `--dry-run` emits the `kind: request` envelope (flat lead catalog +
// project topology) and writes nothing; `--from` schema-gates the
// agent response, projects it onto `plan.yaml.slices[]`, and emits the
// paired `plan.reconcile.{agent,completed}` journal events. JSON shapes
// are pinned by goldens under `tests/fixtures/plan/`; regenerate with
// `REGENERATE_GOLDENS=1 cargo nextest run --test plan`.
// ===================================================================

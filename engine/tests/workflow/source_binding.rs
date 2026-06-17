//! Per-slice source binding flag shape on `plan add` / `plan amend`.
//!
//! The binding flag uses the `<key>=<lead>` wire form, accepting the bare
//! `<key>` shorthand only as sugar for `{ source, lead: <slice.name> }`
//! per workflow §`Slice.sources`.

use crate::support::*;

const W11_PLAN: &str = "\
name: w11
sources:
  intent:
    adapter: intent
    value: \"Demo intent value.\"
  identity-design-notes:
    adapter: documentation
    path: ./docs
slices: []
";

#[test]
fn plan_add_structured_sources_round_trips() {
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    specify_cmd()
        .current_dir(project.root())
        .args([
            "--format",
            "json",
            "plan",
            "add",
            "foo",
            "--sources",
            "identity-design-notes=user-registration",
        ])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        saved.contains("source: identity-design-notes")
            && saved.contains("lead: user-registration"),
        "structured form must round-trip to disk:\n{saved}"
    );
}

#[test]
fn plan_add_bare_source_round_trips() {
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    // Slice name `add-search-filter`; bare `--sources intent` is
    // sugar for `{ source: intent, lead: add-search-filter }`.
    specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "add", "add-search-filter", "--sources", "intent"])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    // Bare form must appear on disk as the YAML scalar `intent`,
    // not the structured `{ source, lead }` mapping.
    assert!(
        saved.contains("  - intent"),
        "bare shorthand must round-trip to the unquoted scalar form:\n{saved}"
    );
    assert!(
        !saved.contains("lead: add-search-filter"),
        "lead=slice.name must collapse to bare form:\n{saved}"
    );
}

#[test]
fn plan_add_structured_lead_differs() {
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "add", "foo", "--sources", "intent=different-candidate"])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        saved.contains("lead: different-candidate"),
        "structured form must stay structured when lead != slice.name:\n{saved}"
    );
}

#[test]
fn add_rejects_dangling_equals() {
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "add", "foo", "--sources", "intent="])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(code, 2, "malformed --sources must exit 2 (argument error), got {code}");
}

#[test]
fn plan_amend_add_source_appends_binding() {
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    add_entry_with(&project, "foo", &["--sources", "intent"]);

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "foo", "--add-source", "identity-design-notes=user-registration"])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        saved.contains("source: identity-design-notes"),
        "amend --add-source must append the binding:\n{saved}"
    );
}

#[test]
fn plan_amend_remove_source_drops_binding() {
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    add_entry_with(
        &project,
        "foo",
        &["--sources", "intent", "--sources", "identity-design-notes=foo"],
    );

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "foo", "--remove-source", "intent"])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(!saved.contains("- intent"), "amend --remove-source must drop the binding:\n{saved}");
    assert!(saved.contains("identity-design-notes"), "non-targeted bindings must remain:\n{saved}");
}

#[test]
fn amend_add_source_duplicate_key_rejected() {
    // A slice binds at most one lead per source key — a duplicate would
    // silently overwrite `evidence/<source>.yaml` at refine time.
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    add_entry_with(&project, "foo", &["--sources", "identity-design-notes=user-registration"]);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args([
            "--format",
            "json",
            "plan",
            "amend",
            "foo",
            "--add-source",
            "identity-design-notes=password-reset",
        ])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(code, 2, "duplicate source key must exit 2 (validation), got {code}");
    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "duplicate-source-key");

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        !saved.contains("password-reset"),
        "rejected --add-source must leave the plan unwritten:\n{saved}"
    );
}

#[test]
fn amend_sources_duplicate_key_rejected() {
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    add_entry_with(&project, "foo", &["--sources", "intent"]);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args([
            "--format",
            "json",
            "plan",
            "amend",
            "foo",
            "--sources",
            "identity-design-notes=lead-a",
            "--sources",
            "identity-design-notes=lead-b",
        ])
        .assert()
        .failure();
    // The wholesale replacement routes through `Plan::amend`'s
    // validate-and-rollback gate, so the generic amend discriminant
    // carries the duplicate-source-key impact text.
    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "plan-amend-validation-failed");
    assert!(
        stderr["message"].as_str().expect("message").contains("more than once"),
        "message must name the duplicate-key violation, got: {stderr}"
    );

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        !saved.contains("lead-a"),
        "rejected --sources must leave the plan unwritten:\n{saved}"
    );
}

#[test]
fn amend_sources_resize_passes() {
    // Same-source re-sizing — replacing the lead bound under an
    // existing key — stays legal; only duplicate keys are rejected.
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    add_entry_with(&project, "foo", &["--sources", "identity-design-notes=user-registration"]);

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "foo", "--sources", "identity-design-notes=password-reset"])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        saved.contains("lead: password-reset") && !saved.contains("user-registration"),
        "re-sizing must replace the bound lead:\n{saved}"
    );
}

#[test]
fn amend_remove_source_unknown_key_errors() {
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    add_entry_with(&project, "foo", &["--sources", "intent"]);

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "amend", "foo", "--remove-source", "no-such-key"])
        .assert()
        .failure();
    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "plan-binding-not-found");
}

#[test]
fn amend_divergence_accepted_writes() {
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    add_pending_entry(&project, "foo");

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "foo", "--divergence", "accepted"])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        saved.contains("divergence: accepted"),
        "amend --divergence accepted must write the field:\n{saved}"
    );
}

#[test]
fn amend_divergence_rejected_writes() {
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    add_pending_entry(&project, "foo");

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "foo", "--divergence", "rejected"])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        saved.contains("divergence: rejected"),
        "amend --divergence rejected must write the field:\n{saved}"
    );
}

#[test]
fn amend_divergence_likely_writes() {
    // divergence and writer-ownership contract: `--divergence likely` is operator-settable from
    // the CLI; the field is byte-identical to the legacy
    // skill-written `divergence: likely` line.
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    add_pending_entry(&project, "foo");

    specify_cmd()
        .current_dir(project.root())
        .args(["plan", "amend", "foo", "--divergence", "likely"])
        .assert()
        .success();

    let saved = fs::read_to_string(project.plan_path()).expect("read plan");
    assert!(
        saved.contains("divergence: likely"),
        "amend --divergence likely must write the field:\n{saved}"
    );
}

#[test]
fn plan_amend_divergence_none_refused() {
    let project = Project::init();
    project.seed_plan(W11_PLAN);

    add_pending_entry(&project, "foo");

    let assert = specify_cmd()
        .current_dir(project.root())
        .args(["--format", "json", "plan", "amend", "foo", "--divergence", "none"])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("exit code");
    assert_eq!(code, 2, "implicit --divergence none must exit 2 (argument error)");
    let stderr = parse_stderr(&assert.get_output().stderr, project.root());
    assert_eq!(stderr["error"], "argument");
}

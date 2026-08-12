//! RFC-86a D9 — the baseline debt projection: `emery debt` walks the
//! baseline specs alone and lists every carried `unknown` / `conflict`
//! requirement with the fields of its self-describing deferral note.
//! The debt-carrying-merge round trip (real notes written by the merge
//! fold) lives in `crates/change/tests/merge_debt.rs`.

use std::fs;
use std::path::Path;

use artifacts::spec::provenance::RequirementStatus;
use jiff::Timestamp;
use mock::invoke::run;
use mock::session::Session;
use project::journal::DeferralOrigin;

/// 2023-11-14T22:13:20Z — ten days after the operator note's date.
fn ts() -> Timestamp {
    Timestamp::from_second(1_700_000_000).expect("valid timestamp")
}

const AUTH_SPEC: &str = "\
# auth\n\n\
### Requirement: password login [unknown]\n\
ID: REQ-001\n\
Sources: []\n\
Status: unknown\n\n\
The login flow handles lockout; behaviour is not evidenced.\n\n\
Note: deferred — origin: operator; change: demo; date: 2023-11-04; reason: lockout deferred to next change\n\n\
### Requirement: session TTL [conflict]\n\
ID: REQ-002\n\
Sources: docs, code\n\
Status: conflict\n\n\
Note: docs says 30 minutes\n\
Note: code says 15 minutes\n\n\
Note: deferred — origin: policy; change: demo; date: 2023-11-14; reason: deferred by gap-policy under epoch 2023-11-14T00:00:00Z\n\n\
### Requirement: greeting text\n\
ID: REQ-003\n\
Sources: docs\n\
Status: agreed\n\n\
The greeting is friendly.\n\n\
### Requirement: retry backoff [divergence]\n\
ID: REQ-004\n\
Sources: docs, code\n\
Status: divergence\n\n\
Docs win; code's shorter backoff is commentary.\n";

/// A carried unknown without a deferral note (merged outside the
/// deferral surface) — still debt, just without provenance detail.
const BILLING_SPEC: &str = "\
# billing\n\n\
### Requirement: refund window [unknown]\n\
ID: REQ-001\n\
Sources: []\n\
Status: unknown\n\n\
The refund window is not evidenced.\n";

fn stage_baseline(specs: &Path) {
    for (domain, text) in [("auth", AUTH_SPEC), ("billing", BILLING_SPEC)] {
        fs::create_dir_all(specs.join(domain)).expect("mkdir domain");
        fs::write(specs.join(domain).join("spec.md"), text).expect("write spec");
    }
}

/// Acceptance 7: each carried row projects with every note field —
/// reason, origin, originating change, and age — agreed and
/// divergence rows are excluded, and a note-less gap row is listed
/// without deferral detail.
#[test]
fn rows_carry_notes() {
    let dir = tempfile::tempdir().expect("tempdir");
    let specs = dir.path().join("specs");
    stage_baseline(&specs);

    let rows = slice::debt::baseline(&specs, ts()).expect("projection");
    assert_eq!(rows.len(), 3, "{rows:?}");

    let unknown = &rows[0];
    assert_eq!((unknown.domain.as_str(), unknown.req.as_str()), ("auth", "REQ-001"));
    assert_eq!(unknown.status, RequirementStatus::Unknown);
    assert_eq!(unknown.summary, "password login");
    let note = unknown.deferral.as_ref().expect("operator note");
    assert_eq!(note.reason, "lockout deferred to next change");
    assert_eq!(note.origin, DeferralOrigin::Operator);
    assert_eq!(note.change, "demo");
    assert_eq!(note.deferred_on, "2023-11-04");
    assert_eq!(note.age_days, 10);

    let conflict = &rows[1];
    assert_eq!((conflict.domain.as_str(), conflict.req.as_str()), ("auth", "REQ-002"));
    assert_eq!(conflict.status, RequirementStatus::Conflict);
    let note = conflict.deferral.as_ref().expect("policy note");
    assert_eq!(note.origin, DeferralOrigin::Policy);
    assert!(note.reason.starts_with("deferred by gap-policy under epoch "), "{}", note.reason);
    assert_eq!(note.age_days, 0);

    let noteless = &rows[2];
    assert_eq!((noteless.domain.as_str(), noteless.req.as_str()), ("billing", "REQ-001"));
    assert!(noteless.deferral.is_none(), "no deferral note to parse: {noteless:?}");
}

/// An empty (or absent) baseline projects cleanly.
#[test]
fn empty_baseline_ok() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = slice::debt::baseline(&dir.path().join("specs"), ts()).expect("missing tree");
    assert!(missing.is_empty(), "{missing:?}");

    fs::create_dir_all(dir.path().join("specs")).expect("mkdir specs");
    let empty = slice::debt::baseline(&dir.path().join("specs"), ts()).expect("empty tree");
    assert!(empty.is_empty(), "{empty:?}");
}

/// A hand-mangled note degrades to a detail-less row instead of
/// failing the read-only projection.
#[test]
fn mangled_note_empty() {
    let dir = tempfile::tempdir().expect("tempdir");
    let specs = dir.path().join("specs");
    fs::create_dir_all(specs.join("auth")).expect("mkdir domain");
    fs::write(
        specs.join("auth").join("spec.md"),
        "### Requirement: password login [unknown]\n\
         ID: REQ-001\n\
         Sources: []\n\
         Status: unknown\n\n\
         Note: deferred — origin: sometimes; change: demo; date: soon; reason: mangled\n",
    )
    .expect("write spec");

    let rows = slice::debt::baseline(&specs, ts()).expect("projection");
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert!(rows[0].deferral.is_none(), "mangled note parses to nothing: {rows:?}");
}

/// The `emery debt` operation over an initialised project: conflicts
/// render separately from unknowns (D6 visibility), each line carrying
/// reason, origin, change, and age.
#[tokio::test]
async fn handler_splits_kinds() {
    let session = Session::scripted("mock", Vec::new());
    stage_baseline(&session.root().join(".emery/specs"));

    let body =
        run::<slice::handlers::Debt, _, _>(session.provider(), slice::handlers::DebtInput {})
            .await
            .expect("debt projects");
    assert_eq!(body.rows.len(), 3, "{:?}", body.rows);

    let mut out = Vec::new();
    project::handler::Render::render(&body, &mut out).expect("render");
    let text = String::from_utf8(out).expect("utf8");
    assert!(text.contains("baseline debt (3 carried rows):"), "{text}");
    // The operation reads the wall clock, so the age is unpinned here;
    // the pinned-clock age arithmetic is covered by the kernel tests.
    assert!(
        text.contains(
            "auth/REQ-001 password login — lockout deferred to next change \
             (operator, change demo, "
        ),
        "{text}"
    );
    assert!(text.contains("billing/REQ-001 refund window"), "{text}");
    let unknown_at = text.find("unknown:").expect("unknown heading");
    let conflict_at = text.find("conflict:").expect("conflict heading");
    assert!(unknown_at < conflict_at, "unknowns render before conflicts: {text}");
    assert!(
        text.lines().position(|l| l.contains("auth/REQ-002"))
            > text.lines().position(|l| l.trim() == "conflict:"),
        "the conflict row renders under the conflict heading: {text}"
    );
}

/// The review-prose section `plan author` folds into `change.md` — the
/// same inventory, markdown-framed, absent when the baseline is clean.
#[test]
fn md_renders_inventory() {
    assert!(slice::debt::markdown(&[]).is_none(), "a clean baseline renders no section");

    let dir = tempfile::tempdir().expect("tempdir");
    let specs = dir.path().join("specs");
    stage_baseline(&specs);
    let rows = slice::debt::baseline(&specs, ts()).expect("projection");

    let section = slice::debt::markdown(&rows).expect("debt section");
    assert!(section.starts_with("## Carried debt\n"), "{section}");
    assert!(
        section.contains(
            "- auth/REQ-001 password login — lockout deferred to next change \
             (operator, change demo, 10 days)"
        ),
        "{section}"
    );
    let unknowns_at = section.find("Unknowns:").expect("unknowns heading");
    let conflicts_at = section.find("Conflicts:").expect("conflicts heading");
    assert!(unknowns_at < conflicts_at, "unknowns render before conflicts: {section}");
    assert!(section.contains("- auth/REQ-002 session TTL — deferred by gap-policy"), "{section}");
}

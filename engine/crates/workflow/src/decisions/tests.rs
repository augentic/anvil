use std::fs;

use super::*;

fn ts() -> Timestamp {
    "2026-06-02T00:00:00Z".parse().expect("timestamp")
}

/// Author a slice-form decision file under `<slice>/decisions/<slug>.md`.
fn author(slice_dir: &Path, slug: &str, status: &str, supersedes: &[&str]) {
    let dir = slice_dir.join("decisions");
    fs::create_dir_all(&dir).expect("mkdir decisions");
    let sup = if supersedes.is_empty() {
        String::new()
    } else {
        format!("supersedes: [{}]\n", supersedes.join(", "))
    };
    let body = format!(
        "---\nslug: {slug}\nstatus: {status}\n{sup}---\n# Title for {slug}\n\n\
         ## Context\nc\n\n## Decision\nd\n\n## Consequences\ne\n"
    );
    fs::write(dir.join(format!("{slug}.md")), body).expect("write slice decision");
}

fn read_baseline_dir(project_dir: &Path) -> Vec<BaselineDecision> {
    read_baseline(&Layout::new(project_dir).decisions_dir()).expect("read baseline")
}

#[test]
fn no_decisions_is_noop() {
    let project = tempfile::tempdir().expect("tempdir");
    let slice = project.path().join(".specify/slices/s1");
    fs::create_dir_all(&slice).expect("mkdir slice");
    let assigned = promote(&slice, project.path(), "s1", ts()).expect("promote");
    assert!(assigned.is_empty());
    assert!(!Layout::new(project.path()).decisions_dir().exists());
}

#[test]
fn fresh_add_assigns_dec_0001() {
    let project = tempfile::tempdir().expect("tempdir");
    let slice = project.path().join(".specify/slices/s1");
    author(&slice, "use-postgres", "accepted", &[]);

    let assigned = promote(&slice, project.path(), "identity-service", ts()).expect("promote");
    assert_eq!(assigned, vec!["DEC-0001"]);

    let baseline = read_baseline_dir(project.path());
    assert_eq!(baseline.len(), 1);
    assert_eq!(baseline[0].id(), "DEC-0001");
    assert_eq!(baseline[0].record.slug, "use-postgres");
    assert_eq!(baseline[0].record.status, DecisionStatus::Accepted);
    assert_eq!(baseline[0].record.slice.as_deref(), Some("identity-service"));
    assert_eq!(baseline[0].record.date.as_deref(), Some("2026-06-02"));
    assert_eq!(baseline[0].title.as_deref(), Some("Title for use-postgres"));
    let path = Layout::new(project.path()).decisions_dir().join("DEC-0001-use-postgres.md");
    assert!(path.is_file(), "promoted file at canonical path");
}

#[test]
fn multi_record_ids_assigned_in_slug_order() {
    let project = tempfile::tempdir().expect("tempdir");
    let slice = project.path().join(".specify/slices/s1");
    author(&slice, "zebra", "accepted", &[]);
    author(&slice, "alpha", "accepted", &[]);
    author(&slice, "mango", "rejected", &[]);

    let assigned = promote(&slice, project.path(), "s1", ts()).expect("promote");
    assert_eq!(assigned, vec!["DEC-0001", "DEC-0002", "DEC-0003"]);

    let baseline = read_baseline_dir(project.path());
    let by_id: Vec<(&str, &str)> =
        baseline.iter().map(|b| (b.id(), b.record.slug.as_str())).collect();
    assert_eq!(by_id, vec![("DEC-0001", "alpha"), ("DEC-0002", "mango"), ("DEC-0003", "zebra")]);
}

#[test]
fn ids_continue_from_existing_baseline() {
    let project = tempfile::tempdir().expect("tempdir");
    let slice = project.path().join(".specify/slices/s1");

    author(&slice, "first", "accepted", &[]);
    promote(&slice, project.path(), "s1", ts()).expect("promote 1");

    // Second slice adds another record; id continues from max + 1.
    let slice2 = project.path().join(".specify/slices/s2");
    author(&slice2, "second", "accepted", &[]);
    let assigned = promote(&slice2, project.path(), "s2", ts()).expect("promote 2");
    assert_eq!(assigned, vec!["DEC-0002"]);
}

#[test]
fn supersede_flips_baseline_record_by_id() {
    let project = tempfile::tempdir().expect("tempdir");
    let slice = project.path().join(".specify/slices/s1");
    author(&slice, "old-store", "accepted", &[]);
    promote(&slice, project.path(), "s1", ts()).expect("promote 1");

    let slice2 = project.path().join(".specify/slices/s2");
    author(&slice2, "new-store", "accepted", &["DEC-0001"]);
    let assigned = promote(&slice2, project.path(), "s2", ts()).expect("promote 2");
    assert_eq!(assigned, vec!["DEC-0002"]);

    let baseline = read_baseline_dir(project.path());
    let old = baseline.iter().find(|b| b.id() == "DEC-0001").expect("old present");
    assert_eq!(old.record.status, DecisionStatus::Superseded);
    assert_eq!(old.record.superseded_by.as_deref(), Some("DEC-0002"));
    // Body is preserved verbatim.
    assert!(old.body.contains("Title for old-store"));
}

#[test]
fn supersede_flips_baseline_record_by_slug() {
    let project = tempfile::tempdir().expect("tempdir");
    let slice = project.path().join(".specify/slices/s1");
    author(&slice, "old-store", "accepted", &[]);
    promote(&slice, project.path(), "s1", ts()).expect("promote 1");

    let slice2 = project.path().join(".specify/slices/s2");
    author(&slice2, "new-store", "accepted", &["old-store"]);
    promote(&slice2, project.path(), "s2", ts()).expect("promote 2");

    let baseline = read_baseline_dir(project.path());
    let old = baseline.iter().find(|b| b.record.slug == "old-store").expect("old present");
    assert_eq!(old.record.status, DecisionStatus::Superseded);
    assert_eq!(old.record.superseded_by.as_deref(), Some("DEC-0002"));
}

#[test]
fn supersede_earlier_sibling_in_same_merge() {
    let project = tempfile::tempdir().expect("tempdir");
    let slice = project.path().join(".specify/slices/s1");
    // `alpha` sorts before `beta`; `beta` supersedes the sibling `alpha`.
    author(&slice, "alpha", "accepted", &[]);
    author(&slice, "beta", "accepted", &["alpha"]);

    let assigned = promote(&slice, project.path(), "s1", ts()).expect("promote");
    assert_eq!(assigned, vec!["DEC-0001", "DEC-0002"]);

    let baseline = read_baseline_dir(project.path());
    let alpha = baseline.iter().find(|b| b.record.slug == "alpha").expect("alpha");
    assert_eq!(alpha.record.status, DecisionStatus::Superseded);
    assert_eq!(alpha.record.superseded_by.as_deref(), Some("DEC-0002"));
}

#[test]
fn orphan_supersede_aborts() {
    let project = tempfile::tempdir().expect("tempdir");
    let slice = project.path().join(".specify/slices/s1");
    author(&slice, "new-store", "accepted", &["DEC-9999"]);

    let err = promote(&slice, project.path(), "s1", ts()).expect_err("orphan aborts");
    match err {
        Error::Validation { code, .. } => assert_eq!(code, "decision-supersede-orphan"),
        other => panic!("expected validation error, got {other:?}"),
    }
    // Nothing was written.
    assert!(!Layout::new(project.path()).decisions_dir().join("DEC-0001-new-store.md").exists());
}

#[test]
fn dec_number_parses() {
    assert_eq!(dec_number("DEC-0007"), Some(7));
    assert_eq!(dec_number("DEC-12"), Some(12));
    assert_eq!(dec_number("DEC-"), None);
    assert_eq!(dec_number("REQ-001"), None);
    assert!(is_dec_ref("DEC-0001"));
    assert!(!is_dec_ref("some-slug"));
}

//! RFC-86 S4/S7: shared in-scope membership (D24), retirement of the
//! plan-wide single-active-entry gate (D23), and fact-based advance
//! with no stored status fields (D2 / D11).

use std::collections::BTreeMap;

use jiff::Timestamp;
use mock::session::Session;
use project::config::{Layout, ProjectConfig};
use project::handler::Anchor;
use project::journal::{self, Event, EventKind};
use project::plan::{Entry, Plan, Status, advance_gate, advance_next, in_scope, project_ladders};
use project::slice::SliceMetadata;

fn entry(name: &str) -> Entry {
    Entry {
        name: name.into(),
        project: Some("default".into()),
        depends_on: vec![],
        sources: vec![],
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: project::plan::AuthorityOverride::default(),
    }
}

fn plan(entries: Vec<Entry>) -> Plan {
    Plan {
        name: "test".into(),
        sources: BTreeMap::new(),
        entries,
    }
}

fn meta() -> SliceMetadata {
    SliceMetadata {
        target: "demo@1.0.0".into(),
        created_at: None,
        defined_at: None,
        completed_at: None,
        merged_at: None,
        dropped_at: None,
        drop_reason: None,
        touched_specs: vec![],
        outcome: None,
    }
}

fn write_plan(root: &std::path::Path, plan: &Plan) {
    let yaml = serde_saphyr::to_string(plan).expect("serialize plan");
    std::fs::write(root.join("plan.yaml"), yaml).expect("write plan.yaml");
    std::fs::create_dir_all(root.join(".emery/slices")).expect("slices dir");
}

fn seed_in_progress(root: &std::path::Path, plan_name: &str, slice: &str, seconds: i64) {
    let now = Timestamp::from_second(seconds).expect("timestamp");
    let layout = Layout::new(root);
    journal::append_one(
        layout,
        &Event::new(
            now,
            EventKind::SliceClaimed {
                slice_name: slice.into(),
            },
        ),
    )
    .expect("claim");
    journal::append_one(
        layout,
        &Event::new(
            now,
            EventKind::PlanEntryAdvanced {
                plan_name: plan_name.into(),
                slice_name: slice.into(),
            },
        ),
    )
    .expect("advance fact");
}

#[test]
fn multiple_in_progress_is_not_a_validate_finding() {
    let plan = plan(vec![entry("a"), entry("b")]);
    let findings = project::plan::doctor::doctor(&plan, None, None, None);
    assert!(
        findings.iter().all(|f| f.rule_id.as_deref() != Some("multiple-in-progress")),
        "plan-wide single-active-entry is retired; got {findings:?}"
    );
    let gate = advance_gate(&plan, std::path::Path::new("/no/slices"));
    assert!(
        !diagnostics::has_blocking(&gate),
        "advance must not refuse concurrent in-progress entries: {gate:?}"
    );
}

#[test]
fn in_scope_requires_plan_membership_and_not_dropped() {
    let plan = plan(vec![entry("orders")]);
    let on_plan = &plan.entries[0];

    assert!(in_scope(&plan, on_plan, None), "absent metadata is not dropped");
    assert!(in_scope(&plan, on_plan, Some(&meta())), "live metadata stays in-scope");
    let mut dropped = meta();
    dropped.dropped_at = Some(Timestamp::from_second(1_700_000_000).expect("timestamp"));
    assert!(!in_scope(&plan, on_plan, Some(&dropped)), "dropped_at excludes membership");

    let orphan = entry("ghost");
    assert!(!in_scope(&plan, &orphan, None), "absent from the plan is not in-scope");
}

#[test]
fn advance_starts_second_entry_while_another_is_in_progress() {
    let session = Session::scripted("demo", Vec::new());
    let staged = plan(vec![entry("a"), entry("b")]);
    write_plan(session.root(), &staged);
    seed_in_progress(session.root(), "test", "a", 1_700_000_000);

    let config = ProjectConfig::load(session.root()).expect("project.yaml");
    let now = Timestamp::from_second(1_700_000_001).expect("timestamp");
    let body = advance_next(session.provider(), session.provider().paths(), now, &config)
        .expect("advance sibling");
    assert_eq!(body.advanced.as_deref(), Some("b"));
    assert!(body.active.is_none(), "fresh advance, not a mid-slice resume");

    let loaded = Plan::load(&Layout::new(session.root()).plan_path()).expect("reload plan");
    let yaml = std::fs::read_to_string(session.root().join("plan.yaml")).expect("plan.yaml");
    assert!(!yaml.contains("status:"), "plan.yaml must not carry a stored status field: {yaml}");
    let events =
        project::plan::collect_events(&loaded, Layout::new(session.root())).expect("events");
    let ladders = project_ladders(&loaded, &events);
    let a: project::name::SliceName = "a".into();
    let b: project::name::SliceName = "b".into();
    assert_eq!(ladders.get(&a).copied(), Some(Status::InProgress));
    assert_eq!(ladders.get(&b).copied(), Some(Status::InProgress));
}

#[test]
fn advance_resumes_in_progress_when_no_pending_is_eligible() {
    let session = Session::scripted("demo", Vec::new());
    let staged = plan(vec![
        entry("a"),
        Entry {
            depends_on: vec!["a".into()],
            ..entry("b")
        },
    ]);
    write_plan(session.root(), &staged);
    seed_in_progress(session.root(), "test", "a", 1_700_000_000);

    let config = ProjectConfig::load(session.root()).expect("project.yaml");
    let now = Timestamp::from_second(1_700_000_001).expect("timestamp");
    let body = advance_next(session.provider(), session.provider().paths(), now, &config)
        .expect("resume active");
    assert_eq!(body.active.as_deref(), Some("a"));
    assert_eq!(body.reason, Some(project::plan::AdvanceReason::InProgress));
    assert!(body.advanced.is_none());
}

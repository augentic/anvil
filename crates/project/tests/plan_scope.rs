//! RFC-86 S4: shared in-scope membership (D24) and retirement of the
//! plan-wide single-active-entry gate (D23).

use std::collections::BTreeMap;

use jiff::Timestamp;
use mock::session::Session;
use project::config::{Layout, ProjectConfig};
use project::handler::Anchor;
use project::plan::{Entry, Plan, Status, advance_gate, advance_next, in_scope};
use project::slice::{LifecycleStatus, SliceMetadata};

fn entry(name: &str, status: Status) -> Entry {
    Entry {
        name: name.into(),
        project: Some("default".into()),
        status,
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

fn meta(status: LifecycleStatus) -> SliceMetadata {
    SliceMetadata {
        target: "demo@1.0.0".into(),
        status,
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

#[test]
fn multiple_in_progress_is_not_a_validate_finding() {
    let plan = plan(vec![entry("a", Status::InProgress), entry("b", Status::InProgress)]);
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
    let plan = plan(vec![entry("orders", Status::Pending)]);
    let on_plan = &plan.entries[0];

    assert!(in_scope(&plan, on_plan, None), "absent metadata is not dropped");
    assert!(
        in_scope(&plan, on_plan, Some(&meta(LifecycleStatus::Refined))),
        "refined stays in-scope"
    );
    assert!(
        !in_scope(&plan, on_plan, Some(&meta(LifecycleStatus::Dropped))),
        "dropped excludes membership"
    );

    let orphan = entry("ghost", Status::Pending);
    assert!(!in_scope(&plan, &orphan, None), "absent from the plan is not in-scope");
}

#[test]
fn advance_starts_second_entry_while_another_is_in_progress() {
    let session = Session::scripted("demo", Vec::new());
    let staged = plan(vec![entry("a", Status::InProgress), entry("b", Status::Pending)]);
    write_plan(session.root(), &staged);

    let config = ProjectConfig::load(session.root()).expect("project.yaml");
    let now = Timestamp::from_second(1_700_000_000).expect("timestamp");
    let body = advance_next(session.provider(), session.provider().paths(), now, &config)
        .expect("advance sibling");
    assert_eq!(body.advanced.as_deref(), Some("b"));
    assert!(body.active.is_none(), "fresh advance, not a mid-slice resume");

    let loaded = Plan::load(&Layout::new(session.root()).plan_path()).expect("reload plan");
    let statuses: Vec<(&str, Status)> =
        loaded.entries.iter().map(|e| (e.name.as_str(), e.status)).collect();
    assert_eq!(
        statuses,
        vec![("a", Status::InProgress), ("b", Status::InProgress)],
        "both entries may be in-progress concurrently"
    );
}

#[test]
fn advance_resumes_in_progress_when_no_pending_is_eligible() {
    let session = Session::scripted("demo", Vec::new());
    let staged = plan(vec![
        entry("a", Status::InProgress),
        Entry {
            depends_on: vec!["a".into()],
            ..entry("b", Status::Pending)
        },
    ]);
    write_plan(session.root(), &staged);

    let config = ProjectConfig::load(session.root()).expect("project.yaml");
    let now = Timestamp::from_second(1_700_000_000).expect("timestamp");
    let body = advance_next(session.provider(), session.provider().paths(), now, &config)
        .expect("resume active");
    assert_eq!(body.active.as_deref(), Some("a"));
    assert_eq!(body.reason, Some(project::plan::AdvanceReason::InProgress));
    assert!(body.advanced.is_none());
}

//! RFC-86 S16: typed gap inventory + shared-lead presentation rollup
//! (Gaps / D18 / D19 / D24).

use std::collections::BTreeMap;
use std::path::Path;

use artifacts::spec::provenance::RequirementStatus;
use project::config::Layout;
use project::plan::{
    Entry, GapRow, Plan, SharedLeadRollup, SliceSourceBinding, in_scope, plan_gaps_body,
};
use project::slice::SliceMetadata;
use tempfile::TempDir;

fn entry(name: &str, sources: Vec<SliceSourceBinding>) -> Entry {
    Entry {
        name: name.into(),
        project: Some("default".into()),
        depends_on: vec![],
        sources,
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: project::plan::AuthorityOverride::default(),
        allow_composition_replace: false,
    }
}

fn plan(entries: Vec<Entry>) -> Plan {
    Plan {
        name: "test".into(),
        sources: BTreeMap::new(),
        entries,
    }
}

fn write_meta(slice_dir: &Path, dropped: bool) {
    std::fs::create_dir_all(slice_dir).expect("slice dir");
    let mut meta = String::from("target: demo@1.0.0\n");
    if dropped {
        meta.push_str("dropped-at: \"2024-01-01T00:00:00Z\"\n");
    }
    std::fs::write(slice_dir.join("metadata.yaml"), meta).expect("metadata");
}

fn write_model(slice_dir: &Path, body: &str) {
    std::fs::write(slice_dir.join("model.yaml"), body).expect("model.yaml");
}

#[test]
fn multi_homed_lead_annotates_rows_and_suggests_selectors() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(".emery/slices")).expect("slices");

    let staged = plan(vec![
        entry("auth-login", vec![SliceSourceBinding::structured("docs", "conventions")]),
        entry("payments", vec![SliceSourceBinding::structured("docs", "conventions")]),
    ]);

    let auth = root.join(".emery/slices/auth-login");
    write_meta(&auth, false);
    write_model(
        &auth,
        r#"requirements:
  - id: REQ-003
    title: password-reset path not evidenced
    status: unknown
    sources: [docs]
  - id: REQ-007
    title: "session TTL: docs vs intent (tied)"
    status: conflict
    sources: [docs, intent]
"#,
    );

    let payments = root.join(".emery/slices/payments");
    write_meta(&payments, false);
    write_model(
        &payments,
        r#"requirements:
  - id: REQ-008
    title: reset copy not evidenced
    status: unknown
    sources: [docs]
  - id: REQ-012
    title: "retry budget: docs beat behaviour"
    status: divergence
    sources: [docs]
  - id: REQ-001
    title: agreed checkout path
    status: agreed
    sources: [docs]
"#,
    );

    let body = plan_gaps_body(&staged, Layout::new(root)).expect("gaps");
    assert_eq!(body.plan, "test");
    assert_eq!(body.rows.len(), 4, "agreed excluded; four typed gaps: {body:?}");

    assert_eq!(
        body.rows[0],
        GapRow {
            slice: "auth-login".into(),
            req: "REQ-003".into(),
            status: RequirementStatus::Unknown,
            summary: "password-reset path not evidenced".into(),
            shared_lead: Some("docs:conventions".into()),
        }
    );
    assert_eq!(body.rows[1].req, "REQ-007");
    assert_eq!(body.rows[1].status, RequirementStatus::Conflict);
    // Conflict contributes docs+intent; docs:conventions is multi-homed
    // across unknowns too, so the shared-lead annotation still applies.
    assert_eq!(body.rows[1].shared_lead.as_deref(), Some("docs:conventions"));

    assert_eq!(
        body.rows[2],
        GapRow {
            slice: "payments".into(),
            req: "REQ-008".into(),
            status: RequirementStatus::Unknown,
            summary: "reset copy not evidenced".into(),
            shared_lead: Some("docs:conventions".into()),
        }
    );
    assert_eq!(body.rows[3].req, "REQ-012");
    assert_eq!(body.rows[3].status, RequirementStatus::Divergence);

    assert_eq!(
        body.rollups,
        vec![SharedLeadRollup {
            source: "docs".into(),
            lead: "conventions".into(),
            selectors: vec!["auth-login".into(), "payments".into()],
        }]
    );
}

#[test]
fn dropped_slice_excluded_from_inventory() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(".emery/slices")).expect("slices");

    let staged = plan(vec![
        entry("live", vec![SliceSourceBinding::structured("docs", "conventions")]),
        entry("abandoned", vec![SliceSourceBinding::structured("docs", "conventions")]),
    ]);

    let live = root.join(".emery/slices/live");
    write_meta(&live, false);
    write_model(
        &live,
        r"requirements:
  - id: REQ-001
    title: thin path
    status: unknown
    sources: [docs]
",
    );

    let abandoned = root.join(".emery/slices/abandoned");
    write_meta(&abandoned, true);
    write_model(
        &abandoned,
        r"requirements:
  - id: REQ-009
    title: also thin
    status: unknown
    sources: [docs]
",
    );

    let meta = SliceMetadata::load(&abandoned).expect("load dropped meta");
    assert!(!in_scope(&staged, &staged.entries[1], Some(&meta)));

    let body = plan_gaps_body(&staged, Layout::new(root)).expect("gaps");
    assert_eq!(body.rows.len(), 1);
    assert_eq!(body.rows[0].slice, "live");
    assert_eq!(body.rows[0].req, "REQ-001");
    // Dropped sibling removed the multi-home — no shared-lead rollup.
    assert!(body.rows[0].shared_lead.is_none());
    assert!(body.rollups.is_empty());
}

#[test]
fn unrefined_in_scope_slice_contributes_no_rows() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path();
    std::fs::create_dir_all(root.join(".emery/slices")).expect("slices");

    let staged = plan(vec![entry("pending-work", vec![])]);
    write_meta(&root.join(".emery/slices/pending-work"), false);

    let body = plan_gaps_body(&staged, Layout::new(root)).expect("gaps");
    assert!(body.is_empty());
}

//! Canonical slice-local planning projections (RFC-91 D4): digests
//! move only when data reachable from the leaf's own entry changes.

use std::collections::BTreeMap;

use artifacts::leads::Lead;
use project::adapter::catalog::Pin;
use project::plan::{
    Entry, Plan, Projections, SliceSourceBinding, SourceBinding, TargetBinding, contributing_leads,
};
use project::snapshot::SnapshotId;

fn stub_target() -> TargetBinding {
    TargetBinding::new(
        Pin::emery("mock", semver::Version::new(0, 0, 0)),
        ".",
        SnapshotId::from_digest(&"0".repeat(64)),
    )
}

fn binding_value(adapter: &str, value: &str) -> SourceBinding {
    SourceBinding::intent(Pin::emery(adapter, semver::Version::new(0, 0, 0)), value)
}

fn entry(name: &str, sources: Vec<SliceSourceBinding>) -> Entry {
    let mut entry = Entry::named(name, "default");
    entry.sources = sources;
    entry
}

fn plan(sources: BTreeMap<String, SourceBinding>, entries: Vec<Entry>) -> Plan {
    let mut plan = Plan::named("demo");
    plan.targets.insert("default".into(), stub_target());
    plan.sources = sources;
    plan.entries = entries;
    plan
}

fn lead(source: &str, id: &str, synopsis: &str) -> Lead {
    Lead::new(id, source, synopsis)
}

fn base_plan() -> (Plan, Vec<Lead>) {
    let sources = BTreeMap::from([("intent".to_string(), binding_value("intent", "brief"))]);
    let entries =
        vec![entry("orders-api", vec![SliceSourceBinding::structured("intent", "orders-lead")])];
    let inventory = vec![lead("intent", "orders-lead", "orders endpoint")];
    (plan(sources, entries), inventory)
}

fn compute(plan: &Plan, name: &str, inventory: &[Lead]) -> Projections {
    compute_for(plan, name, inventory, None)
}

fn compute_for(plan: &Plan, name: &str, inventory: &[Lead], target: Option<&str>) -> Projections {
    let entry = plan.entries.iter().find(|e| e.name.as_str() == name).expect("entry");
    let contributing = contributing_leads(entry, inventory).expect("leads");
    Projections::compute(plan, entry, &contributing, target).expect("projections")
}

#[test]
fn digests_are_snapshot_ids() {
    let (plan, inventory) = base_plan();
    let projections = compute(&plan, "orders-api", &inventory);
    for digest in [&projections.entry, &projections.leads, &projections.decomposition] {
        SnapshotId::parse(digest.as_str()).expect("sha256:<64 hex>");
    }
}

#[test]
fn sibling_entry_stable() {
    // Adding a sibling entry (and the source it binds) must not move
    // any of the leaf's three digests.
    let (mut plan, inventory) = base_plan();
    let before = compute(&plan, "orders-api", &inventory);

    plan.sources.insert("docs".into(), binding_value("documentation", "manual"));
    plan.entries
        .push(entry("billing-api", vec![SliceSourceBinding::structured("docs", "billing-lead")]));
    let after = compute(&plan, "orders-api", &inventory);

    assert_eq!(before.entry, after.entry);
    assert_eq!(before.leads, after.leads);
    assert_eq!(before.decomposition, after.decomposition);
}

#[test]
fn unrelated_lead_stable() {
    let (plan, mut inventory) = base_plan();
    let before = compute(&plan, "orders-api", &inventory);
    inventory.push(lead("intent", "other-lead", "unrelated"));
    let after = compute(&plan, "orders-api", &inventory);
    assert_eq!(before.leads, after.leads);
}

#[test]
fn amend_moves_entry() {
    let (mut plan, inventory) = base_plan();
    let before = compute(&plan, "orders-api", &inventory);
    plan.entries[0].description = Some("reworded".into());
    let after = compute(&plan, "orders-api", &inventory);
    assert_ne!(before.entry, after.entry);
    assert_eq!(before.leads, after.leads);
    assert_eq!(before.decomposition, after.decomposition);
}

#[test]
fn amend_moves_leads() {
    let (mut plan, mut inventory) = base_plan();
    inventory.push(lead("intent", "second-lead", "second"));
    let before = compute(&plan, "orders-api", &inventory);
    plan.entries[0].sources.push(SliceSourceBinding::structured("intent", "second-lead"));
    let after = compute(&plan, "orders-api", &inventory);
    assert_ne!(before.entry, after.entry);
    assert_ne!(before.leads, after.leads);
    assert_eq!(before.decomposition, after.decomposition);
}

#[test]
fn amend_moves_decomp() {
    let (mut plan, inventory) = base_plan();
    plan.entries.push(entry("shared-types", vec![]));
    let before = compute(&plan, "orders-api", &inventory);
    plan.entries[0].depends_on.push("shared-types".into());
    let after = compute(&plan, "orders-api", &inventory);
    assert_ne!(before.entry, after.entry);
    assert_eq!(before.leads, after.leads);
    assert_ne!(before.decomposition, after.decomposition);
}

#[test]
fn decomp_transitive() {
    // a → b; giving b a new dependency c enters a's closure and moves
    // only a's decomposition digest.
    let (mut plan, inventory) = base_plan();
    plan.entries.push(entry("b", vec![]));
    plan.entries.push(entry("c", vec![]));
    plan.entries[0].depends_on.push("b".into());
    let before = compute(&plan, "orders-api", &inventory);

    let b = plan.entries.iter_mut().find(|e| e.name.as_str() == "b").expect("b");
    b.depends_on.push("c".into());
    let after = compute(&plan, "orders-api", &inventory);

    assert_ne!(before.decomposition, after.decomposition);
    assert_eq!(before.entry, after.entry);
    assert_eq!(before.leads, after.leads);
}

#[test]
fn lead_edit_moves_leads() {
    let (plan, mut inventory) = base_plan();
    let before = compute(&plan, "orders-api", &inventory);
    inventory[0].synopsis = "orders endpoint with pagination".into();
    let after = compute(&plan, "orders-api", &inventory);
    assert_ne!(before.leads, after.leads);
    assert_eq!(before.entry, after.entry);
}

#[test]
fn bare_binding_lead() {
    let sources = BTreeMap::from([("intent".to_string(), binding_value("intent", "brief"))]);
    let plan = plan(sources, vec![entry("orders-api", vec![SliceSourceBinding::bare("intent")])]);
    let inventory = vec![lead("intent", "orders-api", "orders endpoint")];
    let contributing = contributing_leads(&plan.entries[0], &inventory).expect("leads");
    assert_eq!(contributing.len(), 1);
    assert_eq!(contributing[0].lead, "orders-api");
}

#[test]
fn missing_lead_refuses() {
    let (plan, _) = base_plan();
    let err = contributing_leads(&plan.entries[0], &[]).expect_err("missing lead");
    assert!(err.to_string().contains("leads-lead-unknown"), "{err}");
}

#[test]
fn unbound_source_refuses() {
    let (mut plan, inventory) = base_plan();
    plan.sources.clear();
    let contributing = contributing_leads(&plan.entries[0], &inventory).expect("leads");
    let err =
        Projections::compute(&plan, &plan.entries[0], &contributing, None).expect_err("unbound");
    assert!(err.to_string().contains("plan-projection-source-unbound"), "{err}");
}

#[test]
fn target_moves_entry() {
    // Rebinding or re-pinning the declared target adapter moves the
    // entry digest (staling the manifest); the leads and decomposition
    // digests stay stable.
    let (plan, inventory) = base_plan();
    let before = compute_for(&plan, "orders-api", &inventory, Some("omnia"));
    let after = compute_for(&plan, "orders-api", &inventory, Some("omnia@1.0.0"));
    assert_ne!(before.entry, after.entry);
    assert_eq!(before.leads, after.leads);
    assert_eq!(before.decomposition, after.decomposition);

    let unbound = compute_for(&plan, "orders-api", &inventory, None);
    assert_ne!(unbound.entry, before.entry);
}

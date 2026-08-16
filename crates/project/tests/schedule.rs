//! RFC-96 D2: deterministic work-item identity and the ready-set
//! projection — identity moves with input drift, at most one item per
//! entry, canonical ordering, and the identity-level stale-base
//! requeue.

use std::collections::BTreeMap;
use std::path::Path;

use project::adapter::catalog::Pin;
use project::build_record::BuildRecord;
use project::config::Layout;
use project::plan::{
    Entry, LoopStep, Plan, Projections, SliceSourceBinding, SourceBinding, TargetBinding, WorkItem,
    contributing_leads, dir_cid, layers, ready_set,
};
use project::refinement::{
    Dependency, Inputs, Live, Manifest, Planning, VERSION, empty_digest, file_digest, live_profile,
};
use project::seam::wire::{BuildReport, BuildStatus};
use project::snapshot::{CodePatch, SnapshotId};

fn cid(hex: char) -> SnapshotId {
    SnapshotId::from_digest(&hex.to_string().repeat(64))
}

/// Two-entry plan (`b` depends on `a`), value-carrying source, one
/// target seeded at `cid('0')`.
fn write_plan(root: &Path) -> Plan {
    let layout = Layout::new(root);
    std::fs::create_dir_all(layout.change_root()).expect("change home");
    let mut plan = Plan::named("demo");
    plan.targets.insert(
        "default".into(),
        TargetBinding::new(Pin::parse("emery:mock@0.0.0").expect("pin"), ".", cid('0')),
    );
    plan.sources.insert(
        "docs".into(),
        SourceBinding::intent(Pin::parse("emery:mock-docs@0.0.0").expect("pin"), "The docs."),
    );
    for (name, deps) in [("a", &[][..]), ("b", &["a"][..])] {
        let mut entry = Entry::named(name, "default");
        entry.sources = vec![SliceSourceBinding::structured("docs", name)];
        entry.depends_on = deps.iter().map(|dep| (*dep).into()).collect();
        plan.entries.push(entry);
    }
    plan.save(&layout.plan_path()).expect("plan.yaml");
    write_leads(root, "a-synopsis");
    plan
}

fn write_leads(root: &Path, synopsis: &str) {
    let layout = Layout::new(root);
    artifacts::leads::Leads::parse(&format!(
        "## Lead inventory\n\n\
         ### docs:a\n\n- lead: a\n- source: docs\n- synopsis: {synopsis}\n\n\
         ### docs:b\n\n- lead: b\n- source: docs\n- synopsis: b-synopsis\n",
    ))
    .expect("catalog")
    .write_atomic(&layout.leads_path())
    .expect("leads.md");
}

fn inventory(root: &Path) -> Vec<artifacts::leads::Lead> {
    let layout = Layout::new(root);
    artifacts::leads::Leads::load(&layout.leads_path()).expect("catalog").leads().to_vec()
}

/// Write a manifest that projects `Fresh`: inputs recomputed with the
/// same pure kernels freshness uses, empty bundle (no spec tree yet).
fn write_fresh_manifest(root: &Path, plan: &Plan, name: &str, dependencies: Vec<Dependency>) {
    let layout = Layout::new(root);
    let entry = plan.entries.iter().find(|e| e.name == name).expect("entry");
    let contributing = contributing_leads(entry, &inventory(root)).expect("contributing");
    let planning =
        Projections::compute_with(plan, entry, &contributing, None, None).expect("projections");
    let manifest = Manifest {
        version: VERSION,
        slice: name.into(),
        inputs: Inputs {
            planning: Planning {
                entry: planning.entry,
                leads: planning.leads,
                decomposition: planning.decomposition,
            },
            profile: live_profile(plan, entry),
            observations: empty_digest(),
            target_guidance: empty_digest(),
            baseline_specs: dir_cid(&layout.specs_dir()).expect("baseline"),
            sources: BTreeMap::new(),
            dependencies,
        },
        bundle: vec![],
    };
    let slice_dir = layout.slice_dir(name);
    std::fs::create_dir_all(&slice_dir).expect("slice dir");
    manifest.write(&slice_dir).expect("refinement.yaml");
}

/// Stage a real wave manifest freezing the slice's live refinement
/// digest, then a build record naming it — merge readiness loads the
/// wave and checks member freshness (RFC-96 D7).
fn write_record(root: &Path, name: &str, base: SnapshotId) {
    let layout = Layout::new(root);
    let slice_dir = layout.slice_dir(name);
    std::fs::create_dir_all(&slice_dir).expect("slice dir");
    let refinement =
        file_digest(&slice_dir).expect("digest read").expect("fixture refinement manifest");
    let wave = project::wave::Wave::one_member(
        "default",
        base.clone(),
        name.into(),
        refinement,
        vec![],
        project::wave::EpochRef {
            writer: "local".into(),
            sequence: 0,
        },
    );
    let wave_digest = wave.write(layout).expect("wave manifest");
    let record = BuildRecord::from_capture(
        CodePatch {
            base,
            result: cid('b'),
            touched: vec!["src/main.rs".into()],
        },
        wave_digest,
        BuildReport {
            version: 1,
            slice: name.into(),
            target: "mock@0.0.0".into(),
            status: BuildStatus::Success,
            findings: vec![],
            outputs: vec![],
            ui_surface: None,
            covered: vec![],
        },
        vec![],
    );
    record.write(&slice_dir).expect("build record");
}

fn ready(root: &Path, plan: &Plan) -> Vec<WorkItem> {
    ready_set(plan, Layout::new(root), &[], &inventory(root), &mut Live::new()).expect("ready set")
}

#[test]
fn layer_projection() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let plan = write_plan(tmp.path());
    let layer = layers(&plan);
    assert_eq!(layer.get("a").copied(), Some(0));
    assert_eq!(layer.get("b").copied(), Some(1));
}

#[test]
fn refine_gated_on_preds() {
    // Both manifests missing: only the root is refine-ready — the
    // dependent waits for a fresh (or archived) predecessor manifest.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let plan = write_plan(tmp.path());
    let items = ready(tmp.path(), &plan);
    assert_eq!(items.len(), 1, "one item per dispatchable entry");
    assert_eq!(items[0].slice.as_str(), "a");
    assert_eq!(items[0].phase, LoopStep::Refine);
}

#[test]
fn order_and_progression() {
    // A fresh root becomes a build item; the dependent becomes
    // refine-ready — canonical order is layer-then-plan-order within
    // one target.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let plan = write_plan(tmp.path());
    write_fresh_manifest(tmp.path(), &plan, "a", vec![]);
    let items = ready(tmp.path(), &plan);
    let key: Vec<(&str, LoopStep)> =
        items.iter().map(|item| (item.slice.as_str(), item.phase)).collect();
    assert_eq!(key, [("a", LoopStep::Build), ("b", LoopStep::Refine)]);
}

#[test]
fn identity_moves_with_drift() {
    // The refine item's digest covers the contributing-lead
    // projection: a catalog edit mints a new work-item identity.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let plan = write_plan(tmp.path());
    let before = ready(tmp.path(), &plan)[0].clone();
    write_leads(tmp.path(), "a-synopsis-drifted");
    let after = ready(tmp.path(), &plan)[0].clone();
    assert_eq!(before.slice, after.slice);
    assert_eq!(before.phase, after.phase);
    assert_ne!(before.digest, after.digest, "input drift must move the identity");
}

#[test]
fn stale_base_requeues_build() {
    // A build record whose base matches the accepted frontier is
    // merge-ready; a record whose base moved cannot merge — the
    // projection emits a *build* item under the new frontier.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let plan = write_plan(tmp.path());

    write_fresh_manifest(tmp.path(), &plan, "a", vec![]);
    write_record(tmp.path(), "a", cid('0'));
    let items = ready(tmp.path(), &plan);
    let a = items.iter().find(|item| item.slice.as_str() == "a").expect("item for a");
    assert_eq!(a.phase, LoopStep::Merge);
    let merge_digest = a.digest.clone();

    // Move the record's base off the frontier (a sibling merge moved
    // the accepted CID underneath this record).
    let slice_dir = Layout::new(tmp.path()).slice_dir("a");
    std::fs::remove_dir_all(slice_dir.join("builds")).expect("clear records");
    write_record(tmp.path(), "a", cid('9'));
    let items = ready(tmp.path(), &plan);
    let a = items.iter().find(|item| item.slice.as_str() == "a").expect("item for a");
    assert_eq!(a.phase, LoopStep::Build, "stale base requeues as a build item");
    assert_ne!(a.digest, merge_digest, "the requeue is a new identity, not a retry");
}

#[test]
fn retracted_wave_requeues() {
    // A member re-refined after the wave froze stales the frozen
    // binding: the whole uncommitted wave retracts and the projection
    // emits a build item, never the merge (RFC-96 D7).
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let plan = write_plan(tmp.path());
    write_fresh_manifest(tmp.path(), &plan, "a", vec![]);
    write_record(tmp.path(), "a", cid('0'));
    let items = ready(tmp.path(), &plan);
    let a = items.iter().find(|item| item.slice.as_str() == "a").expect("item for a");
    assert_eq!(a.phase, LoopStep::Merge, "fresh frozen wave is merge-ready");

    // Re-refine the member: catalog drift moves the manifest bytes,
    // so the live digest no longer matches the frozen binding.
    write_leads(tmp.path(), "a-synopsis-drifted");
    write_fresh_manifest(tmp.path(), &plan, "a", vec![]);
    let items = ready(tmp.path(), &plan);
    let a = items.iter().find(|item| item.slice.as_str() == "a").expect("item for a");
    assert_eq!(a.phase, LoopStep::Build, "the retracted wave requeues as a build");
}

#[test]
fn fixture_manifest_fresh() {
    // The fixture manifest really projects Fresh — guard the fixture
    // against silent drift in the recompute set.
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let plan = write_plan(tmp.path());
    write_fresh_manifest(tmp.path(), &plan, "a", vec![]);
    let layout = Layout::new(tmp.path());
    let entry = plan.entries.iter().find(|e| e.name == "a").expect("entry");
    let freshness = project::refinement::freshness(layout, &plan, entry, &inventory(tmp.path()))
        .expect("freshness");
    let digest = file_digest(&layout.slice_dir("a")).expect("digest").expect("manifest present");
    assert_eq!(freshness, project::refinement::Freshness::Fresh { digest });
}

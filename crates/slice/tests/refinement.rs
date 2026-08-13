//! Refinement manifest kernel (RFC-91 D4): assembly, persistence,
//! digest identity, and the freshness projection.

use std::collections::BTreeMap;
use std::path::Path;

use artifacts::discovery::Lead;
use diagnostics::digest::sha256_hex;
use error::Error;
use project::adapter::BuildInputDeclaration;
use project::config::Layout;
use project::plan::{Entry, Plan, SliceSourceBinding, SourceBinding, close_source_pins};
use project::snapshot::SnapshotId;
use slice::refinement::{
    self, Dependency, Freshness, Inputs, Kind, Manifest, Planning, TargetInputs, empty_digest,
};

const SLICE: &str = "orders-api";

fn fixture() -> (tempfile::TempDir, Plan, Vec<Lead>) {
    let root = tempfile::tempdir().expect("tempdir");

    let docs = root.path().join("docs");
    std::fs::create_dir_all(&docs).expect("mkdir docs");
    std::fs::write(docs.join("a.md"), b"alpha").expect("write docs");

    let baseline = root.path().join(".emery/specs/login");
    std::fs::create_dir_all(&baseline).expect("mkdir baseline");
    std::fs::write(baseline.join("spec.md"), b"login baseline").expect("write baseline");

    let slice_dir = root.path().join(".emery/change/slices").join(SLICE);
    std::fs::create_dir_all(slice_dir.join("specs/orders")).expect("mkdir slice");
    std::fs::write(slice_dir.join("proposal.md"), b"proposal").expect("write");
    std::fs::write(slice_dir.join("design.md"), b"design").expect("write");
    std::fs::write(slice_dir.join("tasks.md"), b"tasks").expect("write");
    std::fs::write(slice_dir.join("specs/orders/spec.md"), b"spec").expect("write");
    std::fs::write(slice_dir.join("notes.md"), b"notes").expect("write");

    let mut plan = Plan {
        name: "demo".into(),
        sources: BTreeMap::from([(
            "docs".to_string(),
            SourceBinding {
                adapter: "documentation".into(),
                version: None,
                path: Some("docs".into()),
                value: None,
                cid: None,
            },
        )]),
        entries: vec![Entry {
            name: SLICE.into(),
            project: Some("default".into()),
            depends_on: vec![],
            sources: vec![SliceSourceBinding::structured("docs", "orders-lead")],
            context: vec![],
            description: None,
            divergence: None,
            disagreements: Vec::new(),
            authority_override: project::plan::AuthorityOverride::default(),
            allow_composition_replace: false,
        }],
    };
    close_source_pins(&mut plan, root.path()).expect("close pins");

    let inventory = vec![Lead {
        lead: "orders-lead".into(),
        source: "docs".into(),
        synopsis: "orders endpoint".into(),
        topics: vec![],
    }];
    (root, plan, inventory)
}

fn declarations() -> Vec<BuildInputDeclaration> {
    vec![
        BuildInputDeclaration {
            path: "notes.md".into(),
            required: true,
        },
        BuildInputDeclaration {
            path: "optional-absent.md".into(),
            required: false,
        },
    ]
}

fn guidance() -> SnapshotId {
    SnapshotId::from_digest(&sha256_hex(b"guidance text"))
}

fn assemble(root: &Path, plan: &Plan, inventory: &[Lead], deps: Vec<Dependency>) -> Manifest {
    try_assemble(root, plan, inventory, deps).expect("assemble")
}

fn try_assemble(
    root: &Path, plan: &Plan, inventory: &[Lead], deps: Vec<Dependency>,
) -> Result<Manifest, Error> {
    refinement::assemble(
        Layout::new(root),
        plan,
        &plan.entries[0],
        inventory,
        TargetInputs {
            guidance: guidance(),
            declarations: &declarations(),
            reference: None,
        },
        deps,
    )
}

fn freshness_of(root: &Path, plan: &Plan, inventory: &[Lead]) -> Freshness {
    let entry = plan.entries.iter().find(|e| e.name.as_str() == SLICE).expect("entry");
    refinement::freshness(Layout::new(root), plan, entry, inventory).expect("freshness")
}

fn stale_reasons(freshness: &Freshness) -> &[String] {
    match freshness {
        Freshness::Stale { reasons } => reasons,
        other => panic!("expected stale, got {other:?}"),
    }
}

/// A minimal predecessor manifest — enough to give a dependent a real
/// `refinement.yaml` to bind against.
fn predecessor(slice: &str) -> Manifest {
    Manifest {
        version: 1,
        slice: slice.into(),
        inputs: Inputs {
            planning: Planning {
                entry: empty_digest(),
                leads: empty_digest(),
                decomposition: empty_digest(),
            },
            profile: empty_digest(),
            observations: empty_digest(),
            target_guidance: empty_digest(),
            baseline_specs: empty_digest(),
            sources: BTreeMap::new(),
            dependencies: vec![],
        },
        bundle: vec![],
    }
}

#[test]
fn round_trip_digest() {
    let (root, plan, inventory) = fixture();
    let slice_dir = Layout::new(root.path()).slice_dir(SLICE);

    let manifest = assemble(root.path(), &plan, &inventory, vec![]);
    manifest.write(&slice_dir).expect("write");

    // Load reproduces the assembled value; the on-disk file digest is
    // the refinement identity everything downstream binds.
    assert_eq!(Manifest::load(&slice_dir).expect("load"), manifest);
    let digest =
        refinement::file_digest(&slice_dir).expect("file digest").expect("manifest present");
    SnapshotId::parse(digest.as_str()).expect("sha256:<64 hex>");

    match freshness_of(root.path(), &plan, &inventory) {
        Freshness::Fresh { digest: fresh } => assert_eq!(fresh, digest),
        other => panic!("expected fresh, got {other:?}"),
    }
}

#[test]
fn bundle_input_set() {
    let (root, plan, inventory) = fixture();
    let manifest = assemble(root.path(), &plan, &inventory, vec![]);

    let shape: Vec<(&str, Kind)> =
        manifest.bundle.iter().map(|entry| (entry.path.as_str(), entry.kind)).collect();
    assert_eq!(
        shape,
        vec![
            ("proposal.md", Kind::Proposal),
            ("design.md", Kind::Design),
            ("tasks.md", Kind::Tasks),
            ("specs/orders/spec.md", Kind::Spec),
            ("notes.md", Kind::Additional),
        ]
    );
}

#[test]
fn empty_optional_ids() {
    let (root, plan, inventory) = fixture();
    let manifest = assemble(root.path(), &plan, &inventory, vec![]);
    assert_eq!(manifest.inputs.profile, empty_digest());
    assert_eq!(manifest.inputs.observations, empty_digest());
    assert_eq!(manifest.inputs.target_guidance, guidance());
    assert_eq!(manifest.inputs.sources.get("docs"), plan.sources["docs"].cid.as_ref());
}

#[test]
fn missing_artifact_refuses() {
    let (root, plan, inventory) = fixture();
    let slice_dir = Layout::new(root.path()).slice_dir(SLICE);
    std::fs::remove_file(slice_dir.join("tasks.md")).expect("rm");
    let err = try_assemble(root.path(), &plan, &inventory, vec![]).expect_err("missing tasks");
    assert!(err.to_string().contains("slice-refinement-input-missing"), "{err}");
}

#[test]
fn missing_spec_refuses() {
    let (root, plan, inventory) = fixture();
    let slice_dir = Layout::new(root.path()).slice_dir(SLICE);
    std::fs::remove_dir_all(slice_dir.join("specs")).expect("rm specs");
    let err = try_assemble(root.path(), &plan, &inventory, vec![]).expect_err("no specs");
    assert!(err.to_string().contains("slice-refinement-input-missing"), "{err}");
}

#[test]
fn missing_additional() {
    let (root, plan, inventory) = fixture();
    let slice_dir = Layout::new(root.path()).slice_dir(SLICE);
    std::fs::remove_file(slice_dir.join("notes.md")).expect("rm");
    let err = try_assemble(root.path(), &plan, &inventory, vec![])
        .expect_err("missing required additional");
    assert!(err.to_string().contains("target-build-input-missing"), "{err}");
}

#[test]
fn unclosed_pin_refuses() {
    let (root, mut plan, inventory) = fixture();
    plan.sources.get_mut("docs").expect("docs").cid = None;
    let err = try_assemble(root.path(), &plan, &inventory, vec![]).expect_err("unclosed pin");
    assert!(err.to_string().contains("slice-refinement-pin-missing"), "{err}");
}

#[test]
fn missing_manifest() {
    let (root, plan, inventory) = fixture();
    assert_eq!(freshness_of(root.path(), &plan, &inventory), Freshness::Missing);

    let findings = refinement::findings(SLICE, &Freshness::Missing);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id.as_deref(), Some(refinement::MISSING_CODE));
}

#[test]
fn edited_bundle_file_stales() {
    let (root, plan, inventory) = fixture();
    let slice_dir = Layout::new(root.path()).slice_dir(SLICE);
    assemble(root.path(), &plan, &inventory, vec![]).write(&slice_dir).expect("write");

    std::fs::write(slice_dir.join("design.md"), b"edited").expect("edit");
    let freshness = freshness_of(root.path(), &plan, &inventory);
    let reasons = stale_reasons(&freshness);
    assert!(reasons.iter().any(|r| r.contains("design.md")), "{reasons:?}");

    let findings = refinement::findings(SLICE, &freshness);
    assert!(!findings.is_empty());
    assert!(
        findings.iter().all(|f| f.rule_id.as_deref() == Some(refinement::STALE_CODE)),
        "{findings:?}"
    );
}

#[test]
fn added_spec_domain_stales() {
    let (root, plan, inventory) = fixture();
    let slice_dir = Layout::new(root.path()).slice_dir(SLICE);
    assemble(root.path(), &plan, &inventory, vec![]).write(&slice_dir).expect("write");

    std::fs::create_dir_all(slice_dir.join("specs/billing")).expect("mkdir");
    std::fs::write(slice_dir.join("specs/billing/spec.md"), b"new spec").expect("write");
    let freshness = freshness_of(root.path(), &plan, &inventory);
    let reasons = stale_reasons(&freshness);
    assert!(reasons.iter().any(|r| r.contains("specs/billing/spec.md")), "{reasons:?}");
}

#[test]
fn baseline_move_stales() {
    let (root, plan, inventory) = fixture();
    let slice_dir = Layout::new(root.path()).slice_dir(SLICE);
    assemble(root.path(), &plan, &inventory, vec![]).write(&slice_dir).expect("write");

    std::fs::write(root.path().join(".emery/specs/login/spec.md"), b"moved").expect("edit");
    let freshness = freshness_of(root.path(), &plan, &inventory);
    assert!(
        stale_reasons(&freshness).iter().any(|r| r.contains("baseline-specs")),
        "{freshness:?}"
    );
}

#[test]
fn source_tree_drift_stales() {
    let (root, plan, inventory) = fixture();
    let slice_dir = Layout::new(root.path()).slice_dir(SLICE);
    assemble(root.path(), &plan, &inventory, vec![]).write(&slice_dir).expect("write");

    std::fs::write(root.path().join("docs/a.md"), b"changed").expect("edit");
    let freshness = freshness_of(root.path(), &plan, &inventory);
    assert!(stale_reasons(&freshness).iter().any(|r| r.contains("source `docs`")), "{freshness:?}");
}

#[test]
fn predecessor_binding() {
    let (root, plan, inventory) = fixture();
    let layout = Layout::new(root.path());
    let slice_dir = layout.slice_dir(SLICE);
    let shared_dir = layout.slice_dir("shared-types");

    let shared = predecessor("shared-types");
    shared.write(&shared_dir).expect("write shared");
    let dependency = Dependency {
        slice: "shared-types".into(),
        refinement: refinement::file_digest(&shared_dir)
            .expect("file digest")
            .expect("shared manifest present"),
    };
    assemble(root.path(), &plan, &inventory, vec![dependency]).write(&slice_dir).expect("write");
    assert!(matches!(freshness_of(root.path(), &plan, &inventory), Freshness::Fresh { .. }));

    // A re-refined predecessor (different manifest bytes) invalidates
    // the dependent through the recorded dependency digest.
    let mut changed = predecessor("shared-types");
    changed.inputs.target_guidance = guidance();
    changed.write(&shared_dir).expect("rewrite shared");
    let freshness = freshness_of(root.path(), &plan, &inventory);
    assert!(stale_reasons(&freshness).iter().any(|r| r.contains("shared-types")), "{freshness:?}");

    // A missing predecessor manifest makes the dependent stale too.
    std::fs::remove_file(Manifest::path(&shared_dir)).expect("rm shared");
    let freshness = freshness_of(root.path(), &plan, &inventory);
    assert!(
        stale_reasons(&freshness).iter().any(|r| r.contains("no refinement manifest")),
        "{freshness:?}"
    );
}

#[test]
fn archived_pred_fresh() {
    // A merged (or dropped) predecessor's slice tree moves to
    // `.emery/change/archive/<stamp>-<slice>/` with its manifest; the archived
    // digest satisfies the dependent's pin (RFC-91 D3). The newest
    // archive entry wins.
    let (root, plan, inventory) = fixture();
    let layout = Layout::new(root.path());
    let slice_dir = layout.slice_dir(SLICE);
    let old_dir = layout.archive_dir().join("2026-01-01-shared-types");
    let new_dir = layout.archive_dir().join("2026-02-01-shared-types");
    std::fs::create_dir_all(&old_dir).expect("mkdir old archive");
    std::fs::create_dir_all(&new_dir).expect("mkdir new archive");
    let mut old = predecessor("shared-types");
    old.inputs.target_guidance = guidance();
    old.write(&old_dir).expect("write old");
    predecessor("shared-types").write(&new_dir).expect("write new");

    let dependency = Dependency {
        slice: "shared-types".into(),
        refinement: refinement::file_digest(&new_dir)
            .expect("file digest")
            .expect("archived manifest present"),
    };
    assemble(root.path(), &plan, &inventory, vec![dependency]).write(&slice_dir).expect("write");
    assert!(matches!(freshness_of(root.path(), &plan, &inventory), Freshness::Fresh { .. }));
}

#[test]
fn merged_baseline_fresh() {
    // A plan-local wave commit journals the post-merge baseline; a live
    // tree matching that newest journaled digest is accepted drift, not
    // staleness (RFC-91 D4). Drift past the commit stales again.
    let (root, plan, inventory) = fixture();
    let layout = Layout::new(root.path());
    let slice_dir = layout.slice_dir(SLICE);
    assemble(root.path(), &plan, &inventory, vec![]).write(&slice_dir).expect("write");

    std::fs::write(root.path().join(".emery/specs/login/spec.md"), b"sibling merged")
        .expect("merge baseline");
    let merged = project::plan::dir_cid(&layout.specs_dir()).expect("dir cid");
    let event = project::journal::Event::new(
        jiff::Timestamp::UNIX_EPOCH,
        project::journal::EventKind::TargetMergeWaveCommitted {
            target: "demo".into(),
            digest: "sha256:0000".into(),
            members: vec!["billing-api".into()],
            base: SnapshotId::from_digest(&"a".repeat(64)),
            result: SnapshotId::from_digest(&"b".repeat(64)),
            commit_authorization: project::journal::FactEpochRef {
                writer: "local".into(),
                sequence: 1,
            },
            identity_maps: vec![],
            baseline: Some(merged),
            deferred: vec![],
        },
    );
    project::journal::append_one(layout, &event).expect("journal");
    assert!(matches!(freshness_of(root.path(), &plan, &inventory), Freshness::Fresh { .. }));

    // Drift past the journaled commit is staleness again.
    std::fs::write(root.path().join(".emery/specs/login/spec.md"), b"post-merge drift")
        .expect("drift");
    let freshness = freshness_of(root.path(), &plan, &inventory);
    assert!(
        stale_reasons(&freshness).iter().any(|r| r.contains("baseline-specs")),
        "{freshness:?}"
    );
}

#[test]
fn amend_stales_entry() {
    let (root, mut plan, inventory) = fixture();
    let slice_dir = Layout::new(root.path()).slice_dir(SLICE);
    assemble(root.path(), &plan, &inventory, vec![]).write(&slice_dir).expect("write");

    // Unrelated sibling entry: still fresh.
    plan.entries.push(Entry {
        name: "billing-api".into(),
        project: Some("default".into()),
        depends_on: vec![],
        sources: vec![],
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: project::plan::AuthorityOverride::default(),
        allow_composition_replace: false,
    });
    assert!(matches!(freshness_of(root.path(), &plan, &inventory), Freshness::Fresh { .. }));

    // Amending the leaf's own entry stales through the planning digests.
    plan.entries[0].description = Some("reworded".into());
    let freshness = freshness_of(root.path(), &plan, &inventory);
    assert!(
        stale_reasons(&freshness).iter().any(|r| r.contains("planning `entry`")),
        "{freshness:?}"
    );
}

#[test]
fn mangled_manifest() {
    let (root, plan, inventory) = fixture();
    let slice_dir = Layout::new(root.path()).slice_dir(SLICE);
    std::fs::create_dir_all(&slice_dir).expect("mkdir");
    std::fs::write(Manifest::path(&slice_dir), b"not: [valid").expect("write");
    let freshness = freshness_of(root.path(), &plan, &inventory);
    assert!(
        stale_reasons(&freshness).iter().any(|r| r.contains("does not parse")),
        "{freshness:?}"
    );
}

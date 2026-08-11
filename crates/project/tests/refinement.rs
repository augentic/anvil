//! Golden for the refinement manifest's canonical YAML bytes.
//!
//! The refinement digest is durable identity — it lands in
//! `plan.execute.started` coverage and `Wave.members[].inputs.refinement`
//! — so a serde field rename or ordering change would silently
//! invalidate every recorded digest. Regenerate with
//! `REGENERATE_GOLDENS=1`.

use std::collections::BTreeMap;
use std::path::PathBuf;

use project::refinement::{
    BundleEntry, Dependency, Inputs, Kind, Manifest, Planning, VERSION, empty_digest,
};
use project::snapshot::SnapshotId;

/// A fixed, fully deterministic manifest value: every digest is a
/// constant, so the golden pins serialization shape only.
fn manifest() -> Manifest {
    let digest = |n: u8| SnapshotId::from_digest(&format!("{:064x}", u64::from(n)));
    Manifest {
        version: VERSION,
        slice: "orders-api".into(),
        inputs: Inputs {
            planning: Planning {
                entry: digest(1),
                leads: digest(2),
                decomposition: digest(3),
            },
            profile: empty_digest(),
            observations: empty_digest(),
            target_guidance: digest(4),
            baseline_specs: digest(5),
            sources: BTreeMap::from([
                ("docs".to_string(), digest(6)),
                ("intent".to_string(), digest(7)),
            ]),
            dependencies: vec![Dependency {
                slice: "shared-types".into(),
                refinement: digest(8),
            }],
        },
        bundle: vec![
            BundleEntry {
                path: "proposal.md".into(),
                kind: Kind::Proposal,
                digest: digest(9),
            },
            BundleEntry {
                path: "design.md".into(),
                kind: Kind::Design,
                digest: digest(10),
            },
            BundleEntry {
                path: "tasks.md".into(),
                kind: Kind::Tasks,
                digest: digest(11),
            },
            BundleEntry {
                path: "specs/orders/spec.md".into(),
                kind: Kind::Spec,
                digest: digest(12),
            },
            BundleEntry {
                path: "notes.md".into(),
                kind: Kind::Additional,
                digest: digest(13),
            },
        ],
    }
}

#[test]
fn manifest_golden() {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("answers").join("refinement-manifest.yaml");
    let actual = artifacts::atomic::serialise_yaml(&manifest()).expect("serialise manifest");
    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        std::fs::create_dir_all(path.parent().expect("answers dir")).expect("create answers dir");
        std::fs::write(&path, &actual).expect("regenerate golden");
    }
    let expected = std::fs::read_to_string(&path).expect("read golden");
    assert_eq!(actual, expected, "golden mismatch: {}", path.display());
}

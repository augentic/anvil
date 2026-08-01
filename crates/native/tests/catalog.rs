//! Catalog builder validation, read-only inventory, and [`DynModel`]
//! forwarding over the shared probe implementors.
//!
//! One linked type may implement both axes (the one-axis rule binds
//! component exports, not linked impls). Operation dispatch is the
//! provider's surface — see `tests/provider.rs`.

mod support;

use native::{Catalog, DynModel};
use omnia_guest::Model as _;
use omnia_guest::model::{Format, Request};
use omnia_testkit::model::Scripted;
use project::adapter::Axis;
use support::{BadVersion, Floored, Probe, ProbeV2};

fn model(answers: &[&str]) -> DynModel {
    DynModel::new(Scripted::answers(answers.iter().copied()))
}

// The erased model forwards requests and clones share the backing
// state: two clones drain one FIFO script in call order.
#[tokio::test]
async fn dyn_model_forwards_and_shares() {
    let model = model(&["first", "second"]);
    let clone = model.clone();

    let request = || Request {
        format: Format::Text,
        ..Request::default()
    };
    let first = model.create(request()).await.expect("first scripted answer");
    assert_eq!(first.answer, "first");
    let second = clone.create(request()).await.expect("second scripted answer");
    assert_eq!(second.answer, "second");
}

#[test]
fn entries_and_metadata() {
    let linked = Catalog::builder()
        .source::<Probe>()
        .target::<Probe>()
        .target::<Floored>()
        .build()
        .expect("valid catalog");
    let ids: Vec<String> = linked.entries().iter().map(native::Entry::id).collect();
    assert_eq!(ids, ["source:mock@0.0.0", "target:mock@0.0.0", "target:floored@0.0.0"]);

    let entry = linked.get(Axis::Target, "mock").expect("target entry");
    assert_eq!(entry.version(), "0.0.0");
    assert_eq!(entry.server_name(), "mock-references");
    assert_eq!(entry.metadata().emery_floor, None);
    assert!(!entry.docs().is_empty());

    let floored = linked.get(Axis::Target, "floored").expect("floored entry");
    assert_eq!(floored.metadata().emery_floor.as_deref(), Some("9.9.9"));

    let err = linked.get(Axis::Source, "unknown").expect_err("unlinked refuses");
    assert_eq!(err.variant_str(), "adapter-not-linked");
}

mod validation {
    use super::*;

    #[test]
    fn per_axis_duplicate_refused() {
        let err = Catalog::builder()
            .source::<Probe>()
            .source::<Probe>()
            .build()
            .expect_err("a per-axis duplicate refuses");
        assert!(err.to_string().contains("duplicate"), "{err}");
    }

    #[test]
    fn dual_axis_identity_allowed() {
        // Fixture's intentional dual-axis `mock` shape stays legal.
        Catalog::builder()
            .source::<Probe>()
            .target::<Probe>()
            .build()
            .expect("dual-axis same-identity entries share one shelf");
    }

    #[test]
    fn shelf_conflict_refused() {
        let err = Catalog::builder()
            .source::<Probe>()
            .target::<ProbeV2>()
            .build()
            .expect_err("same name at different versions conflicts on one shelf");
        assert!(err.to_string().contains("conflicting reference-shelf"), "{err}");
    }

    #[test]
    fn non_semver_version_refused() {
        let err = Catalog::builder()
            .target::<BadVersion>()
            .build()
            .expect_err("a non-SemVer identity version refuses");
        assert!(err.to_string().contains("not exact SemVer"), "{err}");
    }
}

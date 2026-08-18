//! Catalog builder validation, read-only inventory, and [`DynModel`]
//! forwarding over the shared probe implementors.

mod support;

use native::{Catalog, DynModel};
use omnia_guest::Model as _;
use omnia_guest::model::{Format, Request};
use omnia_testkit::model::Scripted;
use project::adapter::Axis;
use support::{BadVersion, Floored, Probe};

fn model(answers: &[&str]) -> DynModel {
    DynModel::new(Scripted::answers(answers.iter().copied()))
}

// The erased model forwards requests and clones share the backing
// state: two clones drain one FIFO script in call order.
#[tokio::test]
async fn dyn_model_forwards_shares() {
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
    let linked =
        Catalog::builder().source::<Probe>().source::<Floored>().build().expect("valid catalog");
    let ids: Vec<String> = linked.entries().iter().map(native::Entry::id).collect();
    assert_eq!(ids, ["source:mock@0.0.0", "source:floored@0.0.0"]);

    let entry = linked.get(Axis::Source, "mock").expect("source entry");
    assert_eq!(entry.version(), "0.0.0");
    assert_eq!(entry.server_name(), "mock-references");
    assert_eq!(entry.metadata().emery_floor, None);
    assert!(!entry.docs().is_empty());

    let floored = linked.get(Axis::Source, "floored").expect("floored entry");
    assert_eq!(floored.metadata().emery_floor.as_deref(), Some("9.9.9"));

    let err = linked.get(Axis::Source, "unknown").expect_err("unlinked refuses");
    assert_eq!(err.variant_str(), "adapter-not-linked");
}

mod validation {
    use super::*;

    #[test]
    fn per_axis_duplicate() {
        let err = Catalog::builder()
            .source::<Probe>()
            .source::<Probe>()
            .build()
            .expect_err("a per-axis duplicate refuses");
        assert!(err.to_string().contains("duplicate"), "{err}");
    }

    #[test]
    fn non_semver_version() {
        let err = Catalog::builder()
            .source::<BadVersion>()
            .build()
            .expect_err("a non-SemVer identity version refuses");
        assert!(err.to_string().contains("not exact SemVer"), "{err}");
    }
}

//! Shelf mounting over the linked catalog: one implementor registered
//! on both axes (legal for native impls) shares one docs registry and
//! must mount its `/mcp/<name>` shelf once, not panic the merge.

mod support;

use harness::catalog::Catalog;
use harness::mcp;
use omnia_testkit::model::Scripted;
use support::Probe;

#[test]
fn dual_axis_shelf_mounts_once() {
    let linked: Catalog<Scripted> = Catalog::builder().source::<Probe>().target::<Probe>().build();

    let shelves = mcp::shelves(&linked);
    assert_eq!(shelves.len(), 2, "both axis entries expose the shared shelf");
    assert!(shelves.iter().all(|shelf| shelf.name == "fixture"));

    // Would panic on a duplicate `/mcp/fixture` mount before dedup.
    let _router = mcp::router(&linked);
}

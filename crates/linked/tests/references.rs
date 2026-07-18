//! Linked reference hosting: shelf mounting, the lazy shared loopback
//! listener, per-operation grant routing, no-op document-free
//! catalogs, and awaited shutdown.

mod support;

use linked::{Catalog, DynModel, Provider, ReferenceMode, references};
use omnia_testkit::model::Scripted;
use project::handler::ExecutionPaths;
use project::seam::Source as _;
use support::{Probe, Reflect};

fn model() -> DynModel {
    DynModel::new(Scripted::answers::<&str>([]))
}

#[test]
fn dual_axis_shelf_mounts_once() {
    // One implementor registered on both axes (legal for linked impls)
    // shares one docs registry and must mount its `/mcp/<name>` shelf
    // once, not panic the merge.
    let linked =
        Catalog::builder().source::<Probe>().target::<Probe>().build().expect("valid catalog");

    let shelves = references::shelves(&linked);
    assert_eq!(shelves.len(), 2, "both axis entries expose the shared shelf");
    assert!(shelves.iter().all(|shelf| shelf.name == "fixture"));
    // Shelf identity carries the adapter's version, not the crate's.
    assert!(shelves.iter().all(|shelf| shelf.references.version == "0.0.0"));

    // Would panic on a duplicate `/mcp/fixture` mount before dedup.
    let _router = references::router(&linked);
}

// An online provider starts the shared lazy listener on the first
// operation carrying reference documents and routes the shelf URL into
// the adapter context; shutdown awaits the server task.
#[tokio::test]
async fn online_grant_routing_and_shutdown() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let catalog = Catalog::builder().source::<Reflect>().build().expect("valid catalog");
    let provider = Provider::new(
        ExecutionPaths::operator(tmp.path()),
        model(),
        catalog,
        ReferenceMode::Online,
    );

    let leads = provider.survey("source:reflect".to_string()).await.expect("survey dispatches");
    let url = &leads[0].lead;
    assert!(url.starts_with("http://127.0.0.1:"), "loopback-only listener, got {url}");
    assert!(url.ends_with("/mcp/reflect"), "per-adapter shelf path, got {url}");

    // The served shelf answers over plain TCP: the MCP route exists.
    let base = url.strip_suffix("/mcp/reflect").expect("base");
    let port: u16 = base.rsplit(':').next().expect("port").parse().expect("port number");
    let probe = tokio::net::TcpStream::connect(("127.0.0.1", port)).await;
    assert!(probe.is_ok(), "the listener accepts connections before shutdown");

    provider.shutdown().await;
    let refused = tokio::net::TcpStream::connect(("127.0.0.1", port)).await;
    assert!(refused.is_err(), "shutdown closes the listener");
}

// Offline providers never start a listener; the adapter context
// carries no reference URL.
#[tokio::test]
async fn offline_never_serves() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let catalog = Catalog::builder().source::<Reflect>().build().expect("valid catalog");
    let provider = Provider::new(
        ExecutionPaths::operator(tmp.path()),
        model(),
        catalog,
        ReferenceMode::Offline,
    );

    let leads = provider.survey("source:reflect".to_string()).await.expect("survey dispatches");
    assert_eq!(leads[0].lead, "none");
}

// A document-free catalog keeps online reference hosting a no-op.
#[tokio::test]
async fn no_docs_is_a_no_op() {
    let catalog = Catalog::builder().build().expect("empty catalog");
    let host = references::ReferenceHost::new(catalog);
    let base = host.base().await.expect("a document-free host is a no-op");
    assert_eq!(base, None);
    host.shutdown().await;
}

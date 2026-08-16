//! Native reference hosting: shelf mounting, the lazy shared loopback
//! listener, per-operation grant routing, the always-mounted engine
//! synthesis shelf, and awaited shutdown.

mod support;

use native::{Catalog, DynModel, Provider, ReferenceMode, references};
use omnia_testkit::model::Scripted;
use project::handler::{CachePlacement, ExecutionPaths, Locations};
use project::seam::{Source as _, SourceInput};
use support::{Probe, Reflect};

// Explicit tempdir-rooted layout: hermetic carried locations, no
// environment reads.
fn paths(root: &std::path::Path) -> ExecutionPaths {
    let locations =
        Locations::explicit(root.join("store"), CachePlacement::Parent(root.join("project-cache")));
    ExecutionPaths::new(root, locations)
}

fn model() -> DynModel {
    DynModel::new(Scripted::answers::<&str>([]))
}

#[test]
fn dual_axis_shelf_mounts() {
    // One implementor registered on both axes (legal for linked impls)
    // shares one docs registry and must mount its `/mcp/<name>` shelf
    // once, not panic the merge.
    let linked =
        Catalog::builder().source::<Probe>().target::<Probe>().build().expect("valid catalog");

    let shelves = references::shelves(&linked);
    assert_eq!(shelves.len(), 2, "both axis entries expose the shared shelf");
    assert!(shelves.iter().all(|shelf| shelf.name == "mock"));
    // Shelf identity carries the adapter's version, not the crate's.
    assert!(shelves.iter().all(|shelf| shelf.references.version == "0.0.0"));

    // Would panic on a duplicate `/mcp/mock` mount before dedup.
    let _router = references::router(&linked);
}

// An online provider starts the shared lazy listener on the first
// operation carrying reference documents and routes the shelf URL into
// the adapter context; shutdown awaits the server task.
#[tokio::test]
async fn grant_routing_shutdown() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let catalog = Catalog::builder().source::<Reflect>().build().expect("valid catalog");
    let provider = Provider::new(paths(tmp.path()), model(), catalog, ReferenceMode::Online);

    let leads = provider
        .survey("source:reflect".to_string(), SourceInput::value("main", ""))
        .await
        .expect("survey dispatches");
    let url = &leads.leads[0].lead;
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
    let provider = Provider::new(paths(tmp.path()), model(), catalog, ReferenceMode::Offline);

    let leads = provider
        .survey("source:reflect".to_string(), SourceInput::value("main", ""))
        .await
        .expect("survey dispatches");
    assert_eq!(leads.leads[0].lead, "none");
}

// The engine's synthesis shelf is always mounted (RFC-96 D9), so even
// a document-free catalog serves it: the online provider grants the
// `/mcp/engine/synthesis` URL, the offline provider grants nothing.
#[tokio::test]
async fn synthesis_shelf() {
    use project::seam::Shelf as _;

    let tmp = tempfile::tempdir().expect("tempdir");
    let catalog = Catalog::builder().build().expect("empty catalog");
    let online = Provider::new(paths(tmp.path()), model(), catalog.clone(), ReferenceMode::Online);
    let url = online.synthesis_shelf().await.expect("shelf resolves").expect("shelf served");
    assert!(url.starts_with("http://127.0.0.1:"), "loopback-only listener, got {url}");
    assert!(url.ends_with("/mcp/engine/synthesis"), "engine shelf path, got {url}");
    online.shutdown().await;

    let offline = Provider::new(paths(tmp.path()), model(), catalog, ReferenceMode::Offline);
    let none = offline.synthesis_shelf().await.expect("offline resolves");
    assert_eq!(none, None, "offline providers never bind a socket");
}

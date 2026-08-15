//! The in-process origins seam plus the RFC-104 materialization
//! kernel over it: a coverage location resolves to an observed tree
//! (`observed-cid` provenance) lent as a read-only RFC-87 workspace,
//! and remote fetch trees never outlive the run.

mod support;

use std::io::{Read as _, Write as _};
use std::net::TcpListener;
use std::path::Path;

use native::{Catalog, DynModel, Provider, ReferenceMode};
use omnia_testkit::model::Scripted;
use project::handler::{CachePlacement, ExecutionPaths, Locations};
use project::seam::Workspaces as _;
use support::Probe;

fn provider(root: &Path) -> Provider {
    let locations =
        Locations::explicit(root.join("store"), CachePlacement::Parent(root.join("project-cache")));
    let catalog = Catalog::builder().source::<Probe>().build().expect("valid catalog");
    Provider::new(
        ExecutionPaths::new(root, locations),
        DynModel::new(Scripted::answers(std::iter::empty::<&str>())),
        catalog,
        ReferenceMode::Offline,
    )
}

#[tokio::test]
async fn local_tree() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path().join("definition-home");
    std::fs::create_dir_all(home.join("orders/src")).expect("home tree");
    std::fs::write(home.join("orders/src/main.ts"), "export {};\n").expect("source file");
    let provider = provider(tmp.path());

    let observed = system::materialize::materialize(&provider, &provider, &home, "./orders")
        .await
        .expect("a local tree materializes");
    assert!(observed.cid.as_str().starts_with("sha256:"), "{}", observed.cid.as_str());
    assert_eq!(observed.revision, None, "a local tree has no origin revision");
    let lent = Path::new(&observed.workspace.root).join("src/main.ts");
    assert_eq!(std::fs::read_to_string(lent).expect("lent workspace file"), "export {};\n");

    provider.discard(observed.workspace.id).await.expect("workspace discards");
}

#[tokio::test]
async fn remote_document() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path().join("definition-home");
    std::fs::create_dir_all(&home).expect("home");
    let provider = provider(tmp.path());
    let body = "openapi: 3.1.0\n";
    let locator = format!("http://{}/specs/orders.yaml", serve(body));

    let observed = system::materialize::materialize(&provider, &provider, &home, &locator)
        .await
        .expect("a document origin materializes");
    assert_eq!(observed.revision, None);
    let lent = Path::new(&observed.workspace.root).join("orders.yaml");
    assert_eq!(std::fs::read_to_string(lent).expect("lent document"), body);

    // The transient fetch tree was discarded after the snapshot:
    // nothing `origin-*` survives beneath the workspaces root.
    let workspaces_root = tmp.path().join("workspaces");
    let survivors: Vec<String> = std::fs::read_dir(&workspaces_root)
        .expect("workspaces root")
        .map(|entry| entry.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        !survivors.iter().any(|name| name.starts_with("origin-")),
        "fetch trees never outlive the run: {survivors:?}"
    );
    assert!(survivors.contains(&observed.workspace.id), "the lent workspace remains");

    provider.discard(observed.workspace.id).await.expect("workspace discards");
}

#[tokio::test]
async fn access_failure_typed() {
    // A missing local tree is the typed access failure the survey
    // records as `survey-error: access` — never a panic.
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path().join("definition-home");
    std::fs::create_dir_all(&home).expect("home");
    let provider = provider(tmp.path());

    let err = system::materialize::materialize(&provider, &provider, &home, "./absent")
        .await
        .expect_err("a missing tree fails typed");
    assert_eq!(err.variant_str(), "system-source-access");
}

/// Serve `body` (HTTP 200) for every connection on a fresh local
/// port; the thread loops for the process's life so git's probe
/// connections never starve the document leg.
fn serve(body: &'static str) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut request = [0_u8; 2048];
            drop(stream.read(&mut request));
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            drop(stream.write_all(response.as_bytes()));
        }
    });
    addr
}

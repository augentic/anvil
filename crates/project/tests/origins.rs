//! The origin-fetch kernel: locator classification, the document
//! download leg, and discard's name grammar. The Git clone leg needs
//! a live remote and is covered by the operator-invoked rungs.

use std::io::{Read as _, Write as _};
use std::net::TcpListener;

use error::Error;
use project::origins;

#[test]
fn classify() {
    for remote in ["https://erp.example.com", "http://a/b", "ssh://git@host/r.git", "git@h:r.git"] {
        assert!(origins::is_remote(remote), "{remote}");
    }
    for local in ["./orders", "orders", "/abs/orders", "../up", "file:///tree"] {
        assert!(!origins::is_remote(local), "{local}");
    }
}

#[test]
fn local_refused() {
    let parent = tempfile::tempdir().expect("tempdir");
    let err = origins::fetch(parent.path(), "./orders").expect_err("local paths never fetch");
    assert!(
        matches!(
            &err,
            Error::Diag {
                code: "origin-locator-unsupported",
                ..
            }
        ),
        "{err}"
    );
}

#[test]
fn document_fetch() {
    let parent = tempfile::tempdir().expect("tempdir");
    let body = "openapi: 3.1.0\n";
    let locator = format!("http://{}/specs/orders.yaml", serve(body));

    let fetched = origins::fetch(parent.path(), &locator).expect("document origin fetches");
    assert!(fetched.name.starts_with("origin-"), "{}", fetched.name);
    assert_eq!(fetched.dir, parent.path().join(&fetched.name));
    assert_eq!(fetched.revision, None, "a document origin reports no revision");
    let document = fetched.dir.join("orders.yaml");
    assert_eq!(std::fs::read_to_string(document).expect("downloaded document"), body);

    // Discard removes the tree and is idempotent.
    origins::discard(parent.path(), &fetched.name).expect("discard");
    assert!(!fetched.dir.exists());
    origins::discard(parent.path(), &fetched.name).expect("discard is idempotent");
}

#[test]
fn fetch_refused() {
    // A refused origin is a typed fetch failure, not a panic or an
    // empty tree.
    let parent = tempfile::tempdir().expect("tempdir");
    let locator = format!("http://{}/gone", serve_status("404 Not Found"));
    let err = origins::fetch(parent.path(), &locator).expect_err("a 404 origin fails");
    assert!(
        matches!(
            &err,
            Error::Diag {
                code: "origin-fetch-failed",
                ..
            }
        ),
        "{err}"
    );
}

#[test]
fn discard_grammar() {
    let parent = tempfile::tempdir().expect("tempdir");
    for name in ["..", "workspace-1", "origin-", "origin-a/b", "origin-a..b", ""] {
        let err = origins::discard(parent.path(), name).expect_err(name);
        assert!(
            matches!(
                &err,
                Error::Diag {
                    code: "origin-discard-invalid",
                    ..
                }
            ),
            "{name}: {err}"
        );
    }
}

/// Serve `body` (HTTP 200) for every connection on a fresh local
/// port. Git's `ls-remote` probe consumes connections before the
/// document leg, so the server thread loops for the process's life.
fn serve(body: &'static str) -> std::net::SocketAddr {
    serve_response(move || {
        format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
    })
}

/// Serve an empty-bodied `status` response for every connection.
fn serve_status(status: &'static str) -> std::net::SocketAddr {
    serve_response(move || {
        format!("HTTP/1.1 {status}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
    })
}

fn serve_response(response: impl Fn() -> String + Send + 'static) -> std::net::SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let addr = listener.local_addr().expect("local addr");
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut request = [0_u8; 2048];
            drop(stream.read(&mut request));
            drop(stream.write_all(response().as_bytes()));
        }
    });
    addr
}

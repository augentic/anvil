//! Minimal in-process read-only OCI registry for the pull-on-miss
//! integration tests: anonymous manifest and blob GETs over plain
//! HTTP, serving standard Wasm OCI artifacts under
//! `<host:port>/adapters/<name>:<version>`.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Response, StatusCode, Uri, header};
use launcher::Registry;

/// Route-keyed content: `<repo>/manifests/<tag>` and
/// `<repo>/blobs/<digest>` exactly as they appear under `/v2/`.
type Store = Arc<Mutex<Contents>>;

#[derive(Default)]
struct Contents {
    manifests: HashMap<String, String>,
    blobs: HashMap<String, Vec<u8>>,
}

/// One live in-process registry; dropping it stops the server (the
/// offline-reuse tests rely on that).
pub struct TestRegistry {
    addr: SocketAddr,
    store: Store,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for TestRegistry {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TestRegistry {
    /// Bind an ephemeral port and serve.
    pub async fn serve() -> Self {
        let store = Store::default();
        let app = Router::new().fallback(handle).with_state(Arc::clone(&store));
        let listener =
            tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind test registry");
        let addr = listener.local_addr().expect("registry addr");
        let task = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve test registry");
        });
        Self { addr, store, task }
    }

    /// The repository prefix this server serves.
    pub fn prefix(&self) -> String {
        format!("{}/adapters", self.addr)
    }

    /// A launcher [`Registry`] base over this server.
    pub fn registry(&self) -> Registry {
        Registry::insecure(self.prefix())
    }

    /// Publish one component as a Wasm OCI artifact under
    /// `adapters/<name>:<version>`; returns the manifest digest the
    /// registry resolves for the tag.
    pub fn publish(&self, name: &str, version: &str, component: Vec<u8>) -> String {
        let repo = format!("adapters/{name}");
        let config = b"{}".to_vec();
        let config_digest = digest(&config);
        let layer_digest = digest(&component);
        let manifest = serde_json::json!({
            "schemaVersion": 2,
            "mediaType": "application/vnd.oci.image.manifest.v1+json",
            "config": {
                "mediaType": "application/vnd.wasm.config.v0+json",
                "digest": config_digest,
                "size": config.len(),
            },
            "layers": [{
                "mediaType": "application/wasm",
                "digest": layer_digest,
                "size": component.len(),
            }],
        })
        .to_string();
        let manifest_digest = digest(manifest.as_bytes());

        let mut contents = self.store.lock().expect("registry lock");
        contents.blobs.insert(format!("{repo}/blobs/{config_digest}"), config);
        contents.blobs.insert(format!("{repo}/blobs/{layer_digest}"), component);
        contents.manifests.insert(format!("{repo}/manifests/{version}"), manifest.clone());
        contents.manifests.insert(format!("{repo}/manifests/{manifest_digest}"), manifest);
        drop(contents);
        manifest_digest
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", diagnostics::digest::sha256_hex(bytes))
}

async fn handle(State(store): State<Store>, uri: Uri) -> Response<Body> {
    let Some(rest) = uri.path().strip_prefix("/v2/") else {
        return plain(StatusCode::NOT_FOUND, "outside /v2/");
    };
    // The API-version probe: 200 with no auth challenge keeps the
    // client anonymous.
    if rest.is_empty() {
        return plain(StatusCode::OK, "{}");
    }
    // `<repo>/tags/list` — the tag listing behind `resolve_latest`,
    // derived from the published manifest tags (digest keys excluded).
    if let Some(repo) = rest.strip_suffix("/tags/list") {
        let prefix = format!("{repo}/manifests/");
        let mut tags: Vec<String> = {
            let contents = store.lock().expect("registry lock");
            contents
                .manifests
                .keys()
                .filter_map(|key| key.strip_prefix(&prefix))
                .filter(|tag| !tag.starts_with("sha256:"))
                .map(ToString::to_string)
                .collect()
        };
        if tags.is_empty() {
            return plain(StatusCode::NOT_FOUND, "unknown repository");
        }
        tags.sort();
        let body = serde_json::json!({ "name": repo, "tags": tags }).to_string();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(body))
            .expect("tags response");
    }
    let (manifest, blob) = {
        let contents = store.lock().expect("registry lock");
        (contents.manifests.get(rest).cloned(), contents.blobs.get(rest).cloned())
    };
    if let Some(manifest) = manifest {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/vnd.oci.image.manifest.v1+json")
            .header("Docker-Content-Digest", digest(manifest.as_bytes()))
            .body(Body::from(manifest))
            .expect("manifest response");
    }
    if let Some(blob) = blob {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .body(Body::from(blob))
            .expect("blob response");
    }
    plain(StatusCode::NOT_FOUND, "unknown reference")
}

fn plain(status: StatusCode, body: &'static str) -> Response<Body> {
    Response::builder().status(status).body(Body::from(body)).expect("plain response")
}

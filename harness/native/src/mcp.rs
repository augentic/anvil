//! The per-adapter MCP reference shelves, served natively.
//!
//! Each linked adapter embeds its prose registry and the shared
//! [`References`] `McpServer` (target-neutral);
//! this module mounts one shelf per adapter at `/mcp/<name>` via
//! [`omnia_guest::mcp::router`] — the same `list_docs` / `read_doc`
//! surface the wasm deployment serves, on the dev listener instead.
//! The judgment grant-URL rewrite
//! (`Provider::mcp_base` → `<base>/mcp/<name>`) points spawned
//! agents here; cursor-agent fetches over real HTTP regardless of shim.

use adapter::references::References;
use omnia_guest::axum::Router;

use crate::catalog;

/// One mounted reference shelf: the adapter name (the `/mcp/<name>`
/// path segment and grant name stem) and its embedded doc table.
#[derive(Clone, Copy, Debug)]
pub struct Shelf {
    /// Axis-stripped adapter name (`intent`, `omnia`, …).
    pub name: &'static str,
    /// The adapter's references server over its embedded registry.
    pub references: References,
}

/// Every linked adapter's reference shelf, one per adapter crate.
#[must_use]
pub fn shelves() -> Vec<Shelf> {
    catalog::entries()
        .iter()
        .map(|entry| Shelf {
            name: entry.name(),
            references: References {
                server_name: entry.server_name(),
                version: env!("CARGO_PKG_VERSION"),
                docs: entry.docs(),
            },
        })
        .collect()
}

/// The shelf router: every adapter's MCP references nested at
/// `/mcp/<name>`, ready to merge onto the verb router.
pub fn router() -> Router {
    shelves().into_iter().fold(Router::new(), |router, shelf| {
        router.nest(&format!("/mcp/{}", shelf.name), omnia_guest::mcp::router(shelf.references))
    })
}

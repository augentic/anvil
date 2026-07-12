//! Typed deployment-manifest builder for composed trials.
//!
//! One builder replaces the string-concatenation manifests the
//! subprocess driver and the composed tests used to carry separately:
//! guests (the linked workflow guest plus adapter components by
//! dispatch id), writable mounts, and the `/mcp/<name>` HTTP routes
//! the live cursor backend's spawned agents fetch references from.
//! [`Manifest::render`] serialises the same TOML shape the shipped
//! binary consumes through `specify run --config`.

use std::path::Path;

use serde::Serialize;

/// The WIT interfaces the workflow guest imports over host-mediated
/// dispatch.
pub const LINKS: [&str; 2] = ["specify:adapter/source@0.1.0", "specify:adapter/target@0.1.0"];

/// Every first-party adapter guest in a full composed deployment, by
/// dispatch id.
pub const ADAPTERS: [&str; 8] = [
    "source:intent",
    "source:documentation",
    "source:typescript",
    "source:screenshots",
    "source:captures",
    "target:contracts",
    "target:omnia",
    "target:vectis",
];

/// One composed deployment manifest under construction.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Manifest {
    guest: Vec<Guest>,
    mount: Vec<Mount>,
    #[serde(skip_serializing_if = "Routes::is_empty")]
    route: Routes,
    transport: Transport,
}

#[derive(Debug, Clone, Serialize)]
struct Guest {
    id: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    link: Vec<String>,
    source: Source,
}

#[derive(Debug, Clone, Serialize)]
struct Source {
    path: String,
}

#[derive(Debug, Clone, Serialize)]
struct Mount {
    name: String,
    path: String,
    writable: bool,
}

#[derive(Debug, Clone, Default, Serialize)]
struct Routes {
    http: Vec<HttpRoute>,
}

impl Routes {
    const fn is_empty(&self) -> bool {
        self.http.is_empty()
    }
}

#[derive(Debug, Clone, Serialize)]
struct HttpRoute {
    prefix: String,
    guest: String,
}

#[derive(Debug, Clone, Serialize)]
struct Transport {
    default: String,
}

impl Default for Transport {
    fn default() -> Self {
        Self {
            default: "in-process".to_owned(),
        }
    }
}

impl Manifest {
    /// Start a manifest whose workflow guest is the component at
    /// `source`, linked against the adapter dispatch interfaces.
    #[must_use]
    pub fn workflow(source: &Path) -> Self {
        Self::default().push_guest("workflow", source, LINKS.map(str::to_owned).to_vec())
    }

    /// Add an adapter guest by dispatch id (`source:<name>` or
    /// `target:<name>`). Chainable.
    #[must_use]
    pub fn guest(self, id: &str, source: &Path) -> Self {
        self.push_guest(id, source, Vec::new())
    }

    /// Add a preopen mount. Chainable.
    #[must_use]
    pub fn mount(mut self, name: &str, path: &Path, writable: bool) -> Self {
        self.mount.push(Mount {
            name: name.to_owned(),
            path: path.display().to_string(),
            writable,
        });
        self
    }

    /// Route the HTTP prefix `/mcp/<adapter name>` to `guest`. Chainable.
    #[must_use]
    pub fn mcp_route(mut self, guest: &str) -> Self {
        let name = guest.split_once(':').map_or(guest, |(_, name)| name);
        self.route.http.push(HttpRoute {
            prefix: format!("/mcp/{name}"),
            guest: guest.to_owned(),
        });
        self
    }

    /// Serialise the deployment TOML.
    ///
    /// # Panics
    ///
    /// Panics only if the manifest shape stops being serialisable —
    /// a programming error, not an input error.
    #[must_use]
    pub fn render(&self) -> String {
        toml::to_string(self).expect("the manifest shape serialises as TOML")
    }

    fn push_guest(mut self, id: &str, source: &Path, link: Vec<String>) -> Self {
        self.guest.push(Guest {
            id: id.to_owned(),
            link,
            source: Source {
                path: source.display().to_string(),
            },
        });
        self
    }
}

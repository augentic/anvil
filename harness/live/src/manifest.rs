//! The sandbox deployment manifest for the guest execute loop.
//!
//! Every workflow verb runs through `specify run --config <manifest>
//! -- <verb …>`: the manifest composes the freshly built workflow
//! guest with the sibling checkout's release-built adapter components,
//! the writable `"."` project mount, the writable `/specify-cache`
//! component-cache mount, and one `/mcp/<name>` HTTP route per adapter
//! for the reference shelves the live cursor backend advertises to
//! spawned agents. The shape mirrors the checked-in repo-root
//! `omnia.toml` and the composed test manifests.

use std::fmt::Write as _;
use std::path::Path;

/// Every adapter guest in the composed deployment, by dispatch id.
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

/// Render the sandbox `omnia.toml`.
#[must_use]
pub fn omnia_toml(workflow_wasm: &Path, release: &Path, sandbox: &Path, cache: &Path) -> String {
    let mut manifest = String::new();
    let _ = writeln!(manifest, "[[guest]]\nid = \"workflow\"");
    let _ = writeln!(manifest, "source.path = \"{}\"", workflow_wasm.display());
    let _ = writeln!(
        manifest,
        "link = [\"specify:adapter/source@0.1.0\", \"specify:adapter/target@0.1.0\"]\n"
    );
    for id in ADAPTERS {
        let name = id.split_once(':').map_or(id, |(_, name)| name);
        let _ = writeln!(manifest, "[[guest]]\nid = \"{id}\"");
        let _ = writeln!(manifest, "source.path = \"{}/{name}.wasm\"\n", release.display());
    }
    let _ = writeln!(manifest, "[[mount]]\nname = \".\"");
    let _ = writeln!(manifest, "path = \"{}\"\nwritable = true\n", sandbox.display());
    let _ = writeln!(manifest, "[[mount]]\nname = \"/specify-cache\"");
    let _ = writeln!(manifest, "path = \"{}\"\nwritable = true\n", cache.display());
    for id in ADAPTERS {
        let name = id.split_once(':').map_or(id, |(_, name)| name);
        let _ = writeln!(manifest, "[[route.http]]\nprefix = \"/mcp/{name}\"\nguest = \"{id}\"\n");
    }
    manifest.push_str("[transport]\ndefault = \"in-process\"\n");
    manifest
}

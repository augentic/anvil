//! The harness stays adapter-agnostic: no dependency on any concrete
//! adapter crate. This invariant is what keeps both wrappers
//! declarative — the engine's eval crate instantiates the catalog with
//! its testkit fixture, `specify-dev` (in `augentic/specify-adapters`)
//! with the first-party adapters.

use std::fs;

#[test]
fn no_adapter_crate_dependencies() {
    let manifest = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
        .expect("harness Cargo.toml");

    for path in ["/sources/", "/targets/", "specify-adapters"] {
        assert!(!manifest.contains(path), "harness/Cargo.toml must not path-depend into {path}");
    }
    for name in [
        "captures",
        "contracts",
        "documentation",
        "intent",
        "omnia-target",
        "screenshots",
        "typescript",
        "vectis",
        "testkit",
    ] {
        assert!(
            !manifest.lines().any(|line| line.trim_start().starts_with(&format!("{name}."))
                || line.trim_start().starts_with(&format!("{name} "))
                || line.trim_start().starts_with(&format!("{name}="))),
            "harness/Cargo.toml must not depend on the `{name}` adapter crate"
        );
    }
}

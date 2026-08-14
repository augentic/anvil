//! Fixture-builder self-checks: degenerate intent form and multi-target.

use mock::definition::{Spec, mint};
use project::definition::{INTENT, resolve};

#[test]
fn degenerate_intent_value() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let minted = mint(tmp.path(), &Spec::degenerate("operator brief")).expect("mint");
    let reviewed = resolve(tmp.path(), &minted.wave).expect("resolve");
    assert_eq!(reviewed.digest, minted.digest);
    let scope = &reviewed.handoff.wave.evidence_scopes[0];
    assert_eq!(scope.source, INTENT);
    assert_eq!(scope.value.as_deref(), Some("operator brief"));
    assert!(scope.source_cid.is_none(), "intent carries value, not a CID");
    assert_eq!(reviewed.handoff.wave.targets.len(), 1);
}

#[test]
fn multi_target_home() {
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let minted = mint(tmp.path(), &Spec::multi_target()).expect("mint");
    let reviewed = resolve(tmp.path(), "extract-orders").expect("resolve");
    assert_eq!(reviewed.digest, minted.digest);
    assert_eq!(reviewed.handoff.wave.targets.len(), 2);
    assert_eq!(reviewed.handoff.wave.evidence_scopes.len(), 2);
    assert!(
        reviewed.handoff.wave.evidence_scopes.iter().all(|scope| {
            scope.source_cid.is_some() && scope.value.is_none() && scope.source != INTENT
        }),
        "location-backed scopes carry source-cid only"
    );
}

#[test]
fn eval_case_specs_mint() {
    for spec in [orders_contracts(), omnia_r9k(), orders_omnia()] {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        mint(tmp.path(), &spec).expect("mint");
        resolve(tmp.path(), &spec.wave).expect("resolve");
    }
}

/// Write adapters-repo eval definition homes. Run with
/// `RFC88_MINT=<emery-adapters>/examples/eval/cases cargo test -p emery-mock write_eval_homes -- --ignored`.
#[test]
#[ignore = "fixture generator; set RFC88_MINT to the eval cases root"]
fn write_eval_homes() {
    let root = std::env::var("RFC88_MINT").expect("RFC88_MINT");
    let root = std::path::Path::new(&root);
    mint(&root.join("orders-contracts/definition"), &orders_contracts()).expect("orders");
    mint(&root.join("omnia-r9k/definition"), &omnia_r9k()).expect("r9k");
    mint(&root.join("orders-omnia/definition"), &orders_omnia()).expect("multi");
}

fn orders_contracts() -> Spec {
    let mut spec = Spec::degenerate(
        "Author the API contracts for the orders service described under docs/: the JSON Schema \
         vocabulary plus the HTTP (OpenAPI) surface. Contracts only - no service implementation.",
    );
    spec.definition = "orders".into();
    spec.wave = "deliver".into();
    spec.outcome = "Author the orders API contracts".into();
    spec.targets[0].id = "orders".into();
    spec.targets[0].locator = "product".into();
    spec.targets[0].adapter = "emery:contracts@0.12.0".into();
    spec.scopes[0].adapter = Some("emery:intent@0.12.0".into());
    spec.mappings[0].target = "orders".into();
    spec.scopes.push(mock::definition::location_scope(
        "docs",
        "emery:documentation@0.12.0",
        "docs",
        "docs",
        0x2,
    ));
    spec.mappings.push(project::definition::Mapping {
        source: "docs".into(),
        lead: "docs".into(),
        target: "orders".into(),
    });
    spec
}

fn omnia_r9k() -> Spec {
    let mut spec = Spec::degenerate(
        "Migrate the legacy TypeScript at_r9k_position_adapter \
         (https://bitbucket.org/Propellerhead/at_r9k_position_adapter) under \
         legacy/ into a new Omnia WASM crate with provider-based dependency injection.",
    );
    spec.definition = "at-r9k-position-adapter".into();
    spec.wave = "deliver".into();
    spec.outcome = "Migrate the r9k position adapter into Omnia".into();
    spec.targets[0].id = "service".into();
    spec.targets[0].locator = "product".into();
    spec.targets[0].adapter = "emery:omnia@0.12.0".into();
    spec.scopes[0].adapter = Some("emery:intent@0.12.0".into());
    spec.mappings[0].target = "service".into();
    spec.scopes.push(mock::definition::location_scope(
        "legacy",
        "emery:typescript@0.12.0",
        "legacy/at_r9k_position_adapter",
        "legacy",
        0x6,
    ));
    spec.mappings.push(project::definition::Mapping {
        source: "legacy".into(),
        lead: "legacy".into(),
        target: "service".into(),
    });
    spec
}

fn orders_omnia() -> Spec {
    let mut spec = orders_contracts();
    spec.definition = "orders-and-health".into();
    spec.outcome = "Author orders contracts, then a health crate that consumes them".into();
    spec.targets.push(project::definition::Target {
        id: "health".into(),
        locator: "product/health".into(),
        adapter: "emery:omnia@0.12.0".into(),
    });
    spec.targets[0].locator = "product/orders".into();
    spec.scopes[0].value = Some(
        "Author the orders API contracts from docs/, then a small Omnia health crate that \
         depends on those contracts. Two targets: contracts first, then omnia."
            .into(),
    );
    spec.mappings.push(project::definition::Mapping {
        source: "intent".into(),
        lead: "intent".into(),
        target: "health".into(),
    });
    spec
}

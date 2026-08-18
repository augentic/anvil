//! The exhaustive catalog inventory and the failure profiles, all
//! exercised through the SDK trait surface (no provider hooks).

use adapter::Source;
use adapter::seam::{Authority, ClaimKind, Context, Error, SourceInput};
use mock::{Adapter, Code, Docs, FailExtract, catalog};
use omnia_testkit::model::Scripted;
use project::adapter::Axis;

// The full mock registry, by `(axis, name)` — a new identity or a
// renamed one must update this inventory deliberately.
#[test]
fn inventory() {
    let catalog = catalog();
    let entries: Vec<(Axis, &str)> =
        catalog.entries().iter().map(|entry| (entry.axis(), entry.name())).collect();
    assert_eq!(
        entries,
        vec![
            (Axis::Source, "mock"),
            (Axis::Source, "mock-docs"),
            (Axis::Source, "mock-code"),
            (Axis::Source, "mock-fail-extract"),
        ]
    );
}

fn ctx(id: &str) -> Context<'_> {
    Context {
        adapter_id: id,
        project_root: std::path::Path::new("."),
        mcp_url: None,
        lend: Some(".".to_string()),
    }
}

fn model() -> Scripted {
    Scripted::answers::<&str>([])
}

#[tokio::test]
async fn extract_failure() {
    let err = FailExtract::extract(
        &model(),
        &ctx("source:mock-fail-extract"),
        &SourceInput::value("main", ""),
    )
    .await
    .expect_err("fail-extract fails");
    assert!(matches!(err, Error::Internal(detail) if detail.contains("mock-fail-extract")));
}

// The adversarial pair: docs claims 30 minutes where code observes 15,
// under the matching authorities.
#[tokio::test]
async fn adversarial_pair() {
    let model = model();
    let docs = Docs::extract(&model, &ctx("source:mock-docs"), &SourceInput::value("docs", ""))
        .await
        .expect("docs extract");
    assert_eq!(docs.authority, Authority::Documentation);
    assert!(docs.claims.iter().any(|claim| claim.id.as_deref() == Some("session.timeout")));

    let code = Code::extract(&model, &ctx("source:mock-code"), &SourceInput::value("code", ""))
        .await
        .expect("code extract");
    assert_eq!(code.authority, Authority::Behaviour);
    assert!(code.claims.iter().any(|claim| claim.id.as_deref() == Some("session.timeout")));
}

// The minimal profile: one requirement claim with A8 extras conserved.
#[tokio::test]
async fn minimal_extract() {
    let evidence = Adapter::extract(&model(), &ctx("source:mock"), &SourceInput::value("main", ""))
        .await
        .expect("minimal extract");
    assert_eq!(evidence.claims.len(), 1);
    let claim = &evidence.claims[0];
    assert_eq!(claim.kind, ClaimKind::Requirement);
    assert_eq!(claim.id.as_deref(), Some("greeting.behaviour"));
    assert!(claim.extras.contains_key("statement"), "A8 extras cross the seam");
}

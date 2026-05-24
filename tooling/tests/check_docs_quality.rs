use std::fs;
use std::path::Path;

use tooling::check::{
    MissingDiagramAsset, RfcCitationInDocs, TextPipelineDiagram,
};
use tooling::finding::Check;
use tooling::Context;

fn scaffold_framework_root(root: &Path) {
    fs::create_dir_all(root.join("tooling")).expect("tooling dir");
    fs::write(root.join("tooling/Cargo.toml"), "").expect("tooling manifest");
    fs::create_dir_all(root.join("plugins/spec")).expect("plugins dir");
    fs::create_dir_all(root.join("adapters/sources")).expect("adapters dir");
}

fn ctx_at(root: &Path) -> Context {
    Context::from_framework_root(root).expect("framework root")
}

fn rule_ids(findings: &[tooling::finding::Finding]) -> Vec<&str> {
    findings.iter().map(|f| f.rule_id).collect()
}

#[test]
fn rfc_citation_flags_user_facing_docs() {
    let dir = tempfile::tempdir().expect("tempdir");
    scaffold_framework_root(dir.path());
    fs::create_dir_all(dir.path().join("docs/tutorials")).expect("docs dir");
    fs::write(
        dir.path().join("docs/tutorials/guide.md"),
        "See RFC-5 for the background.\n",
    )
    .expect("write md");

    let findings = RfcCitationInDocs.run(&ctx_at(dir.path()));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "docs.rfc-citation-in-docs");
    assert!(findings[0].message.contains("docs/tutorials/guide.md:1"));
}

#[test]
fn rfc_citation_allows_decision_log() {
    let dir = tempfile::tempdir().expect("tempdir");
    scaffold_framework_root(dir.path());
    fs::create_dir_all(dir.path().join("docs/explanation")).expect("docs dir");
    fs::write(
        dir.path().join("docs/explanation/decision-log.md"),
        "RFC-5 introduced tooling.\n",
    )
    .expect("write md");

    let findings = RfcCitationInDocs.run(&ctx_at(dir.path()));
    assert!(findings.is_empty());
}

#[test]
fn rfc_citation_ignores_link_targets() {
    let dir = tempfile::tempdir().expect("tempdir");
    scaffold_framework_root(dir.path());
    fs::create_dir_all(dir.path().join("docs/tutorials")).expect("docs dir");
    fs::write(
        dir.path().join("docs/tutorials/guide.md"),
        "See [the plan](rfcs/done/rfc-5-plan.md) for history.\n",
    )
    .expect("write md");

    let findings = RfcCitationInDocs.run(&ctx_at(dir.path()));
    assert!(findings.is_empty());
}

#[test]
fn missing_diagram_asset_flags_broken_svg_ref() {
    let dir = tempfile::tempdir().expect("tempdir");
    scaffold_framework_root(dir.path());
    fs::create_dir_all(dir.path().join("docs/reference")).expect("docs dir");
    fs::write(
        dir.path().join("docs/reference/page.md"),
        "![flow](../assets/diagrams/missing.svg)\n",
    )
    .expect("write md");

    let findings = MissingDiagramAsset.run(&ctx_at(dir.path()));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "docs.missing-diagram-asset");
    assert!(findings[0].message.contains("missing.svg"));
}

#[test]
fn missing_diagram_asset_passes_when_svg_exists() {
    let dir = tempfile::tempdir().expect("tempdir");
    scaffold_framework_root(dir.path());
    fs::create_dir_all(dir.path().join("docs/assets/diagrams")).expect("assets dir");
    fs::create_dir_all(dir.path().join("docs/reference")).expect("docs dir");
    fs::write(dir.path().join("docs/assets/diagrams/flow.svg"), "<svg/>").expect("write svg");
    fs::write(
        dir.path().join("docs/reference/page.md"),
        "![flow](../assets/diagrams/flow.svg)\n",
    )
    .expect("write md");

    let findings = MissingDiagramAsset.run(&ctx_at(dir.path()));
    assert!(findings.is_empty());
}

#[test]
fn missing_diagram_asset_skips_http_urls() {
    let dir = tempfile::tempdir().expect("tempdir");
    scaffold_framework_root(dir.path());
    fs::create_dir_all(dir.path().join("docs/reference")).expect("docs dir");
    fs::write(
        dir.path().join("docs/reference/page.md"),
        "![remote](https://example.com/diagram.svg)\n",
    )
    .expect("write md");

    let findings = MissingDiagramAsset.run(&ctx_at(dir.path()));
    assert!(findings.is_empty());
}

#[test]
fn text_pipeline_diagram_flags_arrow_flow() {
    let dir = tempfile::tempdir().expect("tempdir");
    scaffold_framework_root(dir.path());
    fs::create_dir_all(dir.path().join("docs/explanation")).expect("docs dir");
    fs::write(
        dir.path().join("docs/explanation/overview.md"),
        "Pipeline:\n\n```text\nA -> B\n```\n",
    )
    .expect("write md");

    let findings = TextPipelineDiagram.run(&ctx_at(dir.path()));
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "docs.text-pipeline-diagram");
    assert!(findings[0].message.contains("docs/explanation/overview.md"));
}

#[test]
fn text_pipeline_diagram_allows_text_without_arrows() {
    let dir = tempfile::tempdir().expect("tempdir");
    scaffold_framework_root(dir.path());
    fs::create_dir_all(dir.path().join("docs/how-to")).expect("docs dir");
    fs::write(
        dir.path().join("docs/how-to/steps.md"),
        "```text\nplain listing\nno flow\n```\n",
    )
    .expect("write md");

    let findings = TextPipelineDiagram.run(&ctx_at(dir.path()));
    assert!(findings.is_empty());
}

#[test]
fn docs_quality_tolerates_absent_docs_tree() {
    let dir = tempfile::tempdir().expect("tempdir");
    scaffold_framework_root(dir.path());

    let ctx = ctx_at(dir.path());
    assert!(RfcCitationInDocs.run(&ctx).is_empty());
    assert!(MissingDiagramAsset.run(&ctx).is_empty());
    assert!(TextPipelineDiagram.run(&ctx).is_empty());
}

#[test]
fn clean_framework_repo_passes_docs_quality() {
    let ctx = Context::from_manifest_dir(env!("CARGO_MANIFEST_DIR")).expect("framework root");
    let findings: Vec<_> = [
        RfcCitationInDocs.run(&ctx),
        MissingDiagramAsset.run(&ctx),
        TextPipelineDiagram.run(&ctx),
    ]
    .into_iter()
    .flatten()
    .collect();

    assert!(
        findings.is_empty(),
        "unexpected docs findings: {:?}",
        rule_ids(&findings)
    );
}

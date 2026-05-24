//! Ports `tests/cross_repo/sources_test.ts`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::Value as JsonValue;
use walkdir::WalkDir;

use tooling::test_support::{
    framework_context, read_text, read_yaml, validate_cli_schema_or_skip, validate_yaml_file_or_skip,
    walk_source_fixtures, walk_yaml, CliSchemaId,
};

fn ctx() -> tooling::Context {
    framework_context()
}

#[test]
fn sources_fixtures_tree_present() {
    let fixtures = walk_source_fixtures(&ctx());
    assert!(
        !fixtures.is_empty(),
        "expected at least one source fixture under tests/fixtures/sources/"
    );
}

#[test]
fn intent_extract_evidence_schema_validates() {
    let ctx = ctx();
    let path = ctx
        .framework_root()
        .join("tests/fixtures/sources/intent/expected-extract.yaml");
    validate_yaml_file_or_skip(&ctx, CliSchemaId::Evidence, &path).expect("evidence validates");
}

#[test]
fn intent_enumerate_synthesises_candidate_block() {
    let path = ctx()
        .framework_root()
        .join("tests/fixtures/sources/intent/expected-enumerate.md");
    let md = read_text(&path).expect("expected-enumerate.md exists");
    let re = Regex::new(r"(?m)^### \S+").expect("heading regex");
    assert!(
        re.is_match(&md),
        "expected-enumerate.md must contain at least one `### <candidate>` block"
    );
}

#[test]
fn documentation_every_evidence_document_schema_validates() {
    let ctx = ctx();
    let dir = ctx
        .framework_root()
        .join("tests/fixtures/sources/documentation/expected/evidence");
    let paths = walk_yaml(&dir);
    assert!(!paths.is_empty(), "no Evidence docs under {}", dir.display());
    for path in paths {
        validate_yaml_file_or_skip(&ctx, CliSchemaId::Evidence, &path)
            .unwrap_or_else(|err| panic!("{err}"));
    }
}

#[test]
fn code_typescript_evidence_schema_validates_and_discovery_non_empty() {
    let ctx = ctx();
    let evidence_dir = ctx
        .framework_root()
        .join("tests/fixtures/sources/code-typescript/expected/evidence");
    let paths = walk_yaml(&evidence_dir);
    assert!(
        !paths.is_empty(),
        "no Evidence docs under {}",
        evidence_dir.display()
    );
    for path in paths {
        validate_yaml_file_or_skip(&ctx, CliSchemaId::Evidence, &path)
            .unwrap_or_else(|err| panic!("{err}"));
    }

    let discovery = ctx
        .framework_root()
        .join("tests/fixtures/sources/code-typescript/expected/discovery.md");
    let md = read_text(&discovery).expect("expected/discovery.md exists");
    assert!(
        !md.trim().is_empty(),
        "expected/discovery.md is empty"
    );
}

#[test]
fn screenshots_discovery_md_present_and_non_empty() {
    let root = ctx()
        .framework_root()
        .join("tests/fixtures/sources/screenshots");
    let mut seen = 0;
    for entry in WalkDir::new(&root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let file_name = entry.file_name().to_string_lossy();
        if file_name != "discovery.md" {
            continue;
        }
        seen += 1;
        let md = read_text(entry.path()).expect("discovery.md readable");
        assert!(
            !md.trim().is_empty(),
            "empty discovery.md: {}",
            entry.path().display()
        );
    }
    assert!(seen > 0, "expected at least one discovery.md under {}", root.display());
}

#[test]
fn captures_adapter_manifest_is_discoverable() {
    let manifest_path = ctx()
        .framework_root()
        .join("adapters/sources/captures/adapter.yaml");
    assert!(
        manifest_path.is_file(),
        "expected {} to exist; RFC-27 Change 3.1 landed the captures source adapter",
        manifest_path.display()
    );

    let manifest: BTreeMap<String, JsonValue> =
        serde_saphyr::from_str(&read_text(&manifest_path).expect("adapter.yaml readable"))
            .expect("adapter.yaml parses");

    assert_eq!(
        manifest.get("name").and_then(JsonValue::as_str),
        Some("captures")
    );
    assert_eq!(
        manifest.get("axis").and_then(JsonValue::as_str),
        Some("source")
    );

    let briefs = manifest
        .get("briefs")
        .and_then(JsonValue::as_object)
        .expect("briefs mapping");
    assert!(briefs.contains_key("enumerate"), "briefs must include enumerate");
    assert!(briefs.contains_key("extract"), "briefs must include extract");
}

#[test]
fn captures_every_evidence_document_schema_validates_with_example_claims() {
    let ctx = ctx();
    let root = ctx.framework_root().join("tests/fixtures/sources/captures");
    assert!(
        root.is_dir(),
        "expected {} to exist; RFC-27 Change 4.1 landed the golden capture tree",
        root.display()
    );

    let evidence_paths = walk_matching(&root, |path| {
        path.ends_with("expected/evidence.yaml")
    });
    assert!(
        !evidence_paths.is_empty(),
        "no Evidence docs under {}/**/expected/evidence.yaml",
        root.display()
    );

    for path in evidence_paths {
        let data = read_yaml(&path).unwrap_or_else(|err| panic!("{err}"));
        validate_cli_schema_or_skip(&ctx, CliSchemaId::Evidence, &data, &path.display().to_string())
            .unwrap_or_else(|err| panic!("{err}"));

        let authority = data.get("authority").and_then(JsonValue::as_str);
        assert_eq!(
            authority,
            Some("behaviour"),
            "{}: captures emits authority: behaviour by default, got: {authority:?}",
            path.display()
        );

        let adapter = data.get("adapter").and_then(JsonValue::as_str);
        assert_eq!(
            adapter,
            Some("captures"),
            "{}: adapter field must be captures, got: {adapter:?}",
            path.display()
        );

        let claims = data
            .get("claims")
            .and_then(JsonValue::as_array)
            .expect("captures Evidence must carry at least one claim");
        assert!(
            !claims.is_empty(),
            "{}: captures Evidence must carry at least one claim",
            path.display()
        );

        let mut example_claims = 0;
        for claim in claims {
            let Some(claim) = claim.as_object() else {
                continue;
            };
            if claim.get("kind").and_then(JsonValue::as_str) != Some("example") {
                continue;
            }
            example_claims += 1;
            let digest = claim.get("replay-digest").and_then(JsonValue::as_str);
            assert!(
                digest.is_some_and(|value| value.starts_with("sha256:")),
                "{}: example claim {:?} must carry replay-digest: sha256:<hex>, got: {digest:?}",
                path.display(),
                claim.get("claim-id")
            );
        }
        assert!(
            example_claims > 0,
            "{}: captures Evidence must carry at least one kind: example claim",
            path.display()
        );
    }
}

#[test]
fn captures_every_fusion_yaml_schema_validates() {
    let ctx = ctx();
    let root = ctx.framework_root().join("tests/fixtures/sources/captures");
    let fusion_paths = walk_matching(&root, |path| path.ends_with("expected/fusion.yaml"));
    assert!(
        !fusion_paths.is_empty(),
        "no fusion.yaml under {}/**/expected/fusion.yaml",
        root.display()
    );

    for path in fusion_paths {
        validate_yaml_file_or_skip(&ctx, CliSchemaId::Fusion, &path)
            .unwrap_or_else(|err| panic!("{err}"));
    }
}

#[test]
fn captures_discovery_md_names_runtime_as_bound_source_key() {
    let root = ctx()
        .framework_root()
        .join("tests/fixtures/sources/captures");
    let discovery_paths = walk_matching(&root, |path| path.ends_with("expected/discovery.md"));
    assert!(
        !discovery_paths.is_empty(),
        "no discovery.md under {}/**/expected/discovery.md",
        root.display()
    );

    let heading = Regex::new(r"(?m)^### \S+").expect("heading regex");
    let runtime = Regex::new(r"sources:\s*\[\s*runtime\s*\]").expect("runtime source regex");

    for path in discovery_paths {
        let md = read_text(&path).expect("discovery.md readable");
        assert!(
            heading.is_match(&md),
            "{}: must contain at least one `### <candidate>` block",
            path.display()
        );
        assert!(
            runtime.is_match(&md),
            "{}: candidate block must cite the bound `runtime` source key (per workflow Binding example)",
            path.display()
        );
    }
}

fn walk_matching(root: &Path, predicate: impl Fn(&str) -> bool) -> Vec<PathBuf> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .filter(|path| predicate(&path.to_string_lossy()))
        .collect()
}

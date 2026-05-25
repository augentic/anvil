//! Ports `tests/cross_repo/targets_test.ts`.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde_json::Value as JsonValue;
use specify_domain::spec::provenance::{parse_spec_md, validate};

mod common;

use common::{framework_context, read_text, walk_target_fixtures};

fn ctx() -> tooling::Context {
    framework_context()
}

#[test]
fn targets_fixtures_tree_present() {
    let fixtures = walk_target_fixtures(&ctx());
    assert!(
        !fixtures.is_empty(),
        "expected at least one target fixture under tests/fixtures/targets/"
    );
}

#[test]
fn every_input_spec_md_requirement_provenance_block_parses() {
    let fixtures = walk_target_fixtures(&ctx());
    let mut parsed_total = 0;

    for fx in fixtures {
        let spec_path = fx.dir.join("input/spec.md");
        let Some(content) = read_text(&spec_path) else {
            continue;
        };

        let parsed = parse_spec_md(&content);
        let findings = validate(&parsed, &BTreeSet::new());
        if !findings.is_empty() {
            let detail = findings
                .iter()
                .map(|finding| finding.detail.clone())
                .collect::<Vec<_>>()
                .join("; ");
            panic!("{}: {detail}", spec_path.display());
        }
        assert!(
            !parsed.requirements.is_empty(),
            "{}: no requirement blocks parsed (expected at least one)",
            spec_path.display()
        );
        parsed_total += parsed.requirements.len();
    }

    assert!(
        parsed_total > 0,
        "no targets fixture supplied an input/spec.md"
    );
}

#[test]
fn vectis_expected_composition_yaml_well_formed_with_screens() {
    let fixtures = walk_target_fixtures(&ctx());
    let mut seen = 0;

    for fx in fixtures {
        if fx.name != "vectis" {
            continue;
        }
        let composition = fx.dir.join("expected/composition.yaml");
        let Some(content) = read_text(&composition) else {
            continue;
        };
        seen += 1;

        let data: BTreeMap<String, JsonValue> = serde_saphyr::from_str(&content)
            .unwrap_or_else(|err| panic!("{}: did not parse as a YAML mapping: {err}", composition.display()));
        assert!(
            data.contains_key("screens"),
            "{}: missing top-level 'screens'",
            composition.display()
        );
        assert!(
            data.contains_key("version"),
            "{}: missing top-level 'version'",
            composition.display()
        );
    }

    assert!(
        seen > 0,
        "no Vectis fixture exposed expected/composition.yaml (Wave 3.4 requires at least one)"
    );
}

#[test]
fn omnia_expected_crate_has_cargo_toml_and_src_lib_rs() {
    let fixtures = walk_target_fixtures(&ctx());
    let mut seen = 0;

    for fx in fixtures {
        if fx.name != "omnia" {
            continue;
        }
        let cargo = fx.dir.join("expected/crate/Cargo.toml");
        let lib = fx.dir.join("expected/crate/src/lib.rs");
        let Some(cargo_txt) = read_text(&cargo) else {
            continue;
        };
        seen += 1;
        assert!(
            cargo_txt.contains("[package]"),
            "{}: missing [package] table",
            cargo.display()
        );
        assert!(
            read_text(&lib).is_some(),
            "{}: expected sibling src/lib.rs to exist",
            lib.display()
        );
    }

    assert!(
        seen > 0,
        "no Omnia fixture exposed expected/crate/Cargo.toml (Wave 3.1 requires at least one)"
    );
}

#[test]
fn every_shape_evidence_md_has_bullet_items() {
    let fixtures = walk_target_fixtures(&ctx());
    for fx in fixtures {
        let shape = fx.dir.join("expected/shape-evidence.md");
        let Some(content) = read_text(&shape) else {
            continue;
        };
        let has_bullets = content
            .lines()
            .any(|line| line.starts_with("- ") || line.starts_with("* "));
        assert!(
            has_bullets,
            "{}: no bullet items present",
            shape.display()
        );
    }
}

const REQUIRED_FIXTURE_REPLAY_KEYS: [&str; 5] =
    ["passed", "failed", "skipped", "ran-at", "runner"];

#[test]
fn omnia_with_replay_metadata_carries_full_replay_block() {
    let path = ctx()
        .framework_root()
        .join("tests/fixtures/targets/omnia/with-replay/.metadata.yaml");
    let content = read_text(&path)
        .unwrap_or_else(|| panic!("{}: expected metadata.yaml present per RFC-27 Change 4.1", path.display()));

    let data: BTreeMap<String, JsonValue> = serde_saphyr::from_str(&content)
        .unwrap_or_else(|err| panic!("{}: did not parse as a YAML mapping: {err}", path.display()));

    let replay = data
        .get("replay")
        .and_then(JsonValue::as_object)
        .expect("with-replay metadata must carry a replay block");

    for key in REQUIRED_FIXTURE_REPLAY_KEYS {
        assert!(
            replay.contains_key(key),
            "{}: replay block missing required key '{key}'",
            path.display()
        );
    }

    assert!(
        replay.get("passed").and_then(JsonValue::as_i64).is_some(),
        "{}: replay passed must be a number",
        path.display()
    );
    assert!(
        replay.get("failed").and_then(JsonValue::as_i64).is_some(),
        "{}: replay failed must be a number",
        path.display()
    );
}

#[test]
fn omnia_without_replay_metadata_omits_replay() {
    let path = ctx()
        .framework_root()
        .join("tests/fixtures/targets/omnia/without-replay/.metadata.yaml");
    let content = read_text(&path)
        .unwrap_or_else(|| panic!("{}: expected metadata.yaml present per RFC-27 Change 4.1", path.display()));

    let data: BTreeMap<String, JsonValue> = serde_saphyr::from_str(&content)
        .unwrap_or_else(|err| panic!("{}: did not parse as a YAML mapping: {err}", path.display()));

    assert!(
        !data.contains_key("replay"),
        "{}: without-replay metadata MUST omit the replay key entirely — its presence in this fixture would regress workflow D1 'omission is not an error'",
        path.display()
    );
    assert_eq!(
        data.get("target").and_then(JsonValue::as_str),
        Some("omnia"),
        "{}: target field must be 'omnia', got: {:?}",
        path.display(),
        data.get("target")
    );
}

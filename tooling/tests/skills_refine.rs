//! Ports `tests/cross_repo/skills_refine_test.ts`.

use specify_domain::spec::provenance::parse_spec_md;

mod common;

use common::{
    assert_non_empty_markdown, framework_context, read_text, validate_yaml_file_or_skip,
    walk_skill_fixtures, walk_yaml, CliSchemaId,
};

fn ctx() -> tooling::Context {
    framework_context()
}

#[test]
fn skills_refine_fixtures_tree_present() {
    let fixtures = walk_skill_fixtures(&ctx(), "refine");
    assert!(
        !fixtures.is_empty(),
        "expected at least one refine fixture under tests/fixtures/skills/refine/"
    );
}

#[test]
fn skills_refine_every_evidence_input_schema_validates() {
    let ctx = ctx();
    let fixtures = walk_skill_fixtures(&ctx, "refine");
    let mut evidence_files = 0;

    for fx in fixtures {
        let dir = fx.dir.join("inputs").join("evidence");
        if !dir.is_dir() {
            continue;
        }
        for path in walk_yaml(&dir) {
            evidence_files += 1;
            validate_yaml_file_or_skip(&ctx, CliSchemaId::Evidence, &path)
                .unwrap_or_else(|err| panic!("{err}"));
        }
    }

    assert!(
        evidence_files > 0,
        "no Evidence fixtures discovered under refine/*/inputs/evidence/"
    );
}

#[test]
fn skills_refine_every_expected_spec_md_parses_with_closed_status_enum() {
    let fixtures = walk_skill_fixtures(&ctx(), "refine");
    let mut cases_validated = 0;

    for fx in fixtures {
        let spec_path = fx.dir.join("expected").join("spec.md");
        let Some(content) = read_text(&spec_path) else {
            continue;
        };
        cases_validated += 1;

        let parsed = parse_spec_md(&content);
        let mut errors: Vec<String> = parsed
            .findings
            .iter()
            .map(|finding| finding.detail.clone())
            .collect();

        for req in &parsed.requirements {
            if req.id.is_empty() {
                errors.push(format!("requirement '{}' missing ID:", req.name));
            }
            if req.sources_line_absent {
                errors.push(format!("requirement '{}' missing Sources:", req.name));
            }
            if req.status_raw.is_none() {
                errors.push(format!("requirement '{}' missing Status:", req.name));
            } else if req.status.is_none() {
                errors.push(format!(
                    "requirement '{}' Status: '{}' is not in agreed | divergence | conflict | unknown",
                    req.name,
                    req.status_raw.as_deref().unwrap_or_default()
                ));
            }
        }

        if !errors.is_empty() {
            panic!("{}: {}", spec_path.display(), errors.join("; "));
        }
        assert!(
            !parsed.requirements.is_empty(),
            "{}: no requirement blocks parsed",
            spec_path.display()
        );
    }

    assert!(
        cases_validated > 0,
        "no refine fixture exposed an expected/spec.md"
    );
}

#[test]
fn skills_refine_every_expected_artifact_md_is_non_empty() {
    let fixtures = walk_skill_fixtures(&ctx(), "refine");

    for fx in fixtures {
        for name in ["proposal.md", "design.md", "tasks.md"] {
            let path = fx.dir.join("expected").join(name);
            if !path.is_file() {
                continue;
            }
            assert_non_empty_markdown(&path).unwrap_or_else(|err| panic!("{err}"));
        }
    }
}

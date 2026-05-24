//! Ports `tests/cross_repo/skills_loop_test.ts`.

use serde_json::Value as JsonValue;

use tooling::test_support::{
    assert_non_empty_markdown, assert_yaml_mapping, framework_context, walk_skill_fixtures,
};

fn ctx() -> tooling::Context {
    framework_context()
}

fn check_loop_fixture(skill: &str) {
    let fixtures = walk_skill_fixtures(&ctx(), skill);
    assert!(
        !fixtures.is_empty(),
        "expected at least one {skill} fixture under tests/fixtures/skills/{skill}/"
    );

    for fx in fixtures {
        let plan_path = fx.dir.join("input").join("plan.yaml");
        if plan_path.is_file() {
            let data = assert_yaml_mapping(&plan_path).unwrap_or_else(|err| panic!("{err}"));
            assert!(
                data.get("slices").and_then(JsonValue::as_array).is_some(),
                "{}: missing slices[] array",
                plan_path.display()
            );
        }

        let meta_path = fx.dir.join("input").join("slice-metadata.yaml");
        if meta_path.is_file() {
            assert_yaml_mapping(&meta_path).unwrap_or_else(|err| panic!("{err}"));
        }

        for name in [
            "expected-trace.md",
            "expected-stop-hint.md",
            "expected.md",
            "transcript.md",
        ] {
            let path = fx.dir.join(name);
            if path.is_file() {
                assert_non_empty_markdown(&path).unwrap_or_else(|err| panic!("{err}"));
            }
        }
    }
}

#[test]
fn skills_execute_fixture_shape() {
    check_loop_fixture("execute");
}

#[test]
fn skills_build_fixture_shape() {
    check_loop_fixture("build");
}

#[test]
fn skills_merge_fixture_shape() {
    check_loop_fixture("merge");
}

#[test]
fn skills_finalize_fixture_shape() {
    check_loop_fixture("finalize");
}

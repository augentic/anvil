//! Enforce the repo-local framework-quality predicates over the prose
//! and manifest surfaces (links, skills, scenarios, plugins, docs).
//!
//! Run with `cargo test --test framework`. Any finding fails
//! CI. These are the surviving framework checks from the retired
//! `specify lint framework` engine, rehomed as plain cargo tests per
//! the `tests/rust_quality.rs` pattern: policy lives as constants in
//! each module, failures are test failures.

mod links;
mod prose;
mod scenarios;
mod skills;
mod support;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use support::Finding;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_all(root: &Path) -> Vec<Finding> {
    let mut findings = links::run(root);
    findings.extend(skills::run(root));
    findings.extend(scenarios::run(root));
    findings.extend(prose::run(root));
    findings
}

fn render_failures(findings: &[Finding]) -> String {
    let mut grouped: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for finding in findings {
        grouped.entry(finding.check).or_default().push(&finding.message);
    }
    let mut out = String::new();
    for (check, messages) in grouped {
        writeln!(out, "[{check}] {messages:#?}").expect("write to String");
    }
    out
}

#[test]
fn no_framework_findings() {
    let findings = run_all(&repo_root());
    assert!(findings.is_empty(), "framework-quality gates failed:\n{}", render_failures(&findings));
}

fn write(root: &Path, relative: &str, body: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, body).expect("write");
}

fn fired(findings: &[Finding], check: &str) -> bool {
    findings.iter().any(|finding| finding.check == check)
}

// Each ported check must fire on a known-bad fixture; the matrices
// below cover one offending input per check.

#[test]
fn link_checks_fire_on_bad_fixtures() {
    // An unresolved relative link in docs/ fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "docs/guide.md", "See [missing](missing.md).\n");
    let findings = links::run(dir.path());
    assert!(fired(&findings, links::CHECK_LINK_UNRESOLVED));

    // A missing .svg image embed in docs/ fires the diagram check, not
    // the plain-link check.
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "docs/page.md", "![diagram](../assets/gone.svg)\n");
    let findings = links::run(dir.path());
    assert!(fired(&findings, links::CHECK_DIAGRAM_ASSET_MISSING));
    assert!(!fired(&findings, links::CHECK_LINK_UNRESOLVED));

    // A fenced or inline-code link never fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "docs/code.md",
        "```md\n[missing](gone.md)\n```\n\nAnd `[missing](gone.md)` inline.\n",
    );
    assert!(links::run(dir.path()).is_empty());

    // An unknown schemas.specify.dev URL in an adapter tree fires; a
    // registered one is silent.
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "targets/demo/prose/references/guide.md",
        "See https://schemas.specify.dev/demo/unknown.schema.json for the shape.\n",
    );
    assert!(fired(&links::run(dir.path()), links::CHECK_SCHEMA_URL_UNKNOWN));
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "targets/vectis/prose/references/guide.md",
        "See https://schemas.specify.dev/vectis/tokens.schema.json for the shape.\n",
    );
    assert!(!fired(&links::run(dir.path()), links::CHECK_SCHEMA_URL_UNKNOWN));

    // A directive naming an unknown plugin or skill fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "plugins/spec/skills/refine/SKILL.md", "---\nname: specify-refine\n---\n");
    write(dir.path(), "docs/guide.md", "<!-- skill: spec:ghost -->\n");
    assert!(fired(&links::run(dir.path()), links::CHECK_DIRECTIVE_UNRESOLVED));

    // A broken symlink under plugins/ fires.
    #[cfg(unix)]
    check_broken_symlink_fixture();

    // A deployable surface linking into docs/ fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "docs/explanation/why.md", "Why.\n");
    write(
        dir.path(),
        "plugins/spec/skills/refine/SKILL.md",
        "---\nname: specify-refine\n---\n\nSee [why](../../../../docs/explanation/why.md).\n",
    );
    assert!(fired(&links::run(dir.path()), links::CHECK_DOCS_IN_DEPLOYABLE));
}

#[cfg(unix)]
fn check_broken_symlink_fixture() {
    let dir = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(dir.path().join("plugins/spec")).expect("mkdir");
    std::os::unix::fs::symlink("gone.md", dir.path().join("plugins/spec/link.md"))
        .expect("symlink");
    assert!(fired(&links::run(dir.path()), links::CHECK_BROKEN_SYMLINK));
}

/// A schema-valid SKILL.md frontmatter block for fixture trees.
const VALID_SKILL: &str = "---\nname: specify-refine\ndescription: Run the refine phase for one slice. Use when a slice needs refinement.\n---\n\nBody.\n";

#[test]
fn skill_checks_fire_on_bad_fixtures() {
    // A schema-valid, prefix-aligned skill is silent.
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "plugins/spec/skills/refine/SKILL.md", VALID_SKILL);
    assert!(skills::run(dir.path()).is_empty());

    // Missing frontmatter fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "plugins/spec/skills/refine/SKILL.md", "No frontmatter here.\n");
    assert!(fired(&skills::run(dir.path()), skills::CHECK_MISSING_FRONTMATTER));

    // An unknown frontmatter property fires the schema check.
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "plugins/spec/skills/refine/SKILL.md",
        &VALID_SKILL.replace("---\n\nBody.", "bogus: true\n---\n\nBody."),
    );
    assert!(fired(&skills::run(dir.path()), skills::CHECK_SCHEMA_VIOLATION));

    // Two skills sharing one name fire the duplicate check.
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "plugins/spec/skills/refine/SKILL.md", VALID_SKILL);
    write(dir.path(), "plugins/spec/skills/other/SKILL.md", VALID_SKILL);
    assert!(fired(&skills::run(dir.path()), skills::CHECK_DUPLICATE_NAME));

    // A name without its plugin's discovery prefix fires (spec maps to
    // specify- via the override).
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "plugins/spec/skills/refine/SKILL.md",
        &VALID_SKILL.replace("name: specify-refine", "name: spec-refine"),
    );
    assert!(fired(&skills::run(dir.path()), skills::CHECK_NAME_PREFIX));

    // A prose argument-hint fires the grammar check.
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "plugins/spec/skills/refine/SKILL.md",
        &VALID_SKILL.replace("---\n\nBody.", "argument-hint: \"<slice-name>\"\n---\n\nBody."),
    );
    assert!(!fired(&skills::run(dir.path()), skills::CHECK_ARGUMENT_HINT));
    write(
        dir.path(),
        "plugins/spec/skills/refine/SKILL.md",
        &VALID_SKILL.replace("---\n\nBody.", "argument-hint: the slice name\n---\n\nBody."),
    );
    assert!(fired(&skills::run(dir.path()), skills::CHECK_ARGUMENT_HINT));

    // A description that does not open with an approved verb fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "plugins/spec/skills/refine/SKILL.md",
        &VALID_SKILL.replace("description: Run the", "description: Runs the"),
    );
    assert!(fired(&skills::run(dir.path()), skills::CHECK_DESCRIPTION_VERB));

    // A body `## Input` heading fires the restatement check.
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "plugins/spec/skills/refine/SKILL.md",
        &VALID_SKILL.replace("Body.", "## Input\n\nBody."),
    );
    assert!(fired(&skills::run(dir.path()), skills::CHECK_FRONTMATTER_RESTATEMENT));
}

/// A schema-valid scenario frontmatter block keyed by `id`, plus the
/// catalog row and run record that keep the catalog check silent.
fn write_valid_scenario_tree(root: &Path, id: &str) {
    write(
        root,
        &format!("evals/scenarios/{id}.md"),
        &format!(
            "---\nid: {id}\nowner: spec\nkind: skill\nentrypoint: /spec:refine\nstages: [refine, build]\nisolation: fresh-project\n---\n\nBody.\n"
        ),
    );
    write(
        root,
        "evals/scenarios/README.md",
        &format!(
            "| Scenario | File | Status | Gate |\n| --- | --- | --- | --- |\n| One | [`{id}`]({id}.md) | pending | full |\n"
        ),
    );
}

#[test]
fn scenario_checks_fire_on_bad_fixtures() {
    // A schema-valid scenario with a consistent catalog is silent.
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_scenario_tree(dir.path(), "good");
    assert!(scenarios::run(dir.path()).is_empty());

    // A frontmatter block missing required fields fires the schema check.
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_scenario_tree(dir.path(), "bad");
    write(dir.path(), "evals/scenarios/bad.md", "---\nid: bad\n---\n\nBody.\n");
    assert!(fired(&scenarios::run(dir.path()), scenarios::CHECK_SCHEMA_VIOLATION));

    // Two scenarios sharing one id fire the duplicate check (the second
    // lives in a plugin fixture so the catalog stays consistent).
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_scenario_tree(dir.path(), "twin");
    write(
        dir.path(),
        "plugins/spec/skills/refine/fixtures/case/scenario.md",
        "---\nid: twin\nowner: spec\nkind: skill\nentrypoint: /spec:refine\nstages: [refine]\nisolation: fresh-project\n---\n\nBody.\n",
    );
    assert!(fired(&scenarios::run(dir.path()), scenarios::CHECK_DUPLICATE_ID));

    // A body Scenario ID line disagreeing with the frontmatter fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_scenario_tree(dir.path(), "real");
    let path = dir.path().join("evals/scenarios/real.md");
    let content = fs::read_to_string(&path).expect("read");
    fs::write(&path, format!("{content}\nScenario ID: `other`\n")).expect("write");
    assert!(fired(&scenarios::run(dir.path()), scenarios::CHECK_BODY_ID_MISMATCH));

    // A non-contiguous stages list fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_scenario_tree(dir.path(), "stages");
    let path = dir.path().join("evals/scenarios/stages.md");
    let content = fs::read_to_string(&path).expect("read");
    fs::write(&path, content.replace("[refine, build]", "[plan, build]")).expect("write");
    assert!(fired(&scenarios::run(dir.path()), scenarios::CHECK_STAGES_NOT_CONTIGUOUS));

    // An escaping expected-artifact path fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_scenario_tree(dir.path(), "arts");
    let path = dir.path().join("evals/scenarios/arts.md");
    let content = fs::read_to_string(&path).expect("read");
    fs::write(
        &path,
        content.replace(
            "isolation: fresh-project\n",
            "isolation: fresh-project\nexpected-artifacts: ['../escape.txt']\n",
        ),
    )
    .expect("write");
    assert!(fired(&scenarios::run(dir.path()), scenarios::CHECK_ARTIFACT_PATH_UNSAFE));

    // A status-bearing catalog row without its run record fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_scenario_tree(dir.path(), "gate");
    let path = dir.path().join("evals/scenarios/README.md");
    let content = fs::read_to_string(&path).expect("read");
    fs::write(&path, content.replace("| pending |", "| passed |")).expect("write");
    assert!(fired(&scenarios::run(dir.path()), scenarios::CHECK_CATALOG_RUNS_DRIFT));
}

/// A marketplace manifest declaring exactly one `spec` plugin.
const VALID_MARKETPLACE: &str = r#"{
  "name": "test",
  "owner": { "name": "Test Owner", "email": "test@example.com" },
  "metadata": { "description": "Synthetic.", "version": "0.0.0", "pluginRoot": "plugins" },
  "plugins": [
    { "name": "spec", "source": "spec", "description": "Spec plugin." }
  ]
}
"#;

/// Baseline fixture carrying the artifacts the prose checks require.
fn write_valid_prose_tree(root: &Path) {
    write(root, "docs/reference/review-team-protocol.md", "# Review team protocol\n");
    write(
        root,
        "docs/standards/skill-authoring.md",
        "Description cap: 512 characters. Body cap: 200 lines.\n",
    );
}

#[test]
fn prose_checks_fire_on_bad_fixtures() {
    // A complete baseline is silent.
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_prose_tree(dir.path());
    assert!(prose::run(dir.path()).is_empty());

    // An on-disk plugin missing from the manifest fires the drift check.
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_prose_tree(dir.path());
    write(dir.path(), ".cursor-plugin/marketplace.json", VALID_MARKETPLACE);
    write(dir.path(), "plugins/spec/skills/refine/SKILL.md", VALID_SKILL);
    write(dir.path(), "plugins/spec/.cursor-plugin/plugin.json", "{\"name\":\"spec\"}");
    write(dir.path(), "plugins/orphan/skills/x/SKILL.md", VALID_SKILL);
    write(dir.path(), "plugins/orphan/.cursor-plugin/plugin.json", "{\"name\":\"orphan\"}");
    assert!(fired(&prose::run(dir.path()), prose::CHECK_MARKETPLACE_DRIFT));

    // A body cap missing from the standards prose fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_prose_tree(dir.path());
    write(dir.path(), "docs/standards/skill-authoring.md", "Description cap: 512 characters.\n");
    assert!(fired(&prose::run(dir.path()), prose::CHECK_NUMERIC_CAP));

    // A missing canonical review-team-protocol document fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "docs/standards/skill-authoring.md",
        "Description cap: 512 characters. Body cap: 200 lines.\n",
    );
    assert!(fired(&prose::run(dir.path()), prose::CHECK_CANONICAL_MISSING));

    // A two-file reference corpus without a README.md index fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_prose_tree(dir.path());
    write(dir.path(), "targets/demo/prose/references/providers/a.md", "A.\n");
    write(dir.path(), "targets/demo/prose/references/providers/b.md", "B.\n");
    assert!(fired(&prose::run(dir.path()), prose::CHECK_CORPUS_UNINDEXED));

    // A retired design-history citation fires; a standards RFC passes.
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_prose_tree(dir.path());
    write(dir.path(), "docs/explanation/why.md", "Per RFC-5 the loop was split.\n");
    assert!(fired(&prose::run(dir.path()), prose::CHECK_HISTORY_CITATION));
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_prose_tree(dir.path());
    write(dir.path(), "docs/explanation/time.md", "Timestamps follow RFC 3339.\n");
    assert!(!fired(&prose::run(dir.path()), prose::CHECK_HISTORY_CITATION));

    // A ```text fence with a flow arrow in an explanation doc fires.
    let dir = tempfile::tempdir().expect("tempdir");
    write_valid_prose_tree(dir.path());
    write(dir.path(), "docs/explanation/flow.md", "```text\nplan -> refine -> build\n```\n");
    assert!(fired(&prose::run(dir.path()), prose::CHECK_TEXT_DIAGRAM));
}

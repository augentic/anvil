//! Enforce the repo-local framework-quality predicates over the prose
//! and manifest surfaces (links, skills, plugins, docs, naming).
//!
//! Run with `cargo nextest run -p framework`. Any finding fails CI.
//! Policy lives as constants in each module; failures are test failures.

mod boundaries;
mod links;
mod naming;
mod prose;
mod skills;
mod support;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

use support::Finding;

fn repo_root() -> PathBuf {
    // The package manifest lives at tests/framework/, two levels under
    // the repository root the predicates walk.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("tests/framework sits two levels under the repo root")
        .to_path_buf()
}

fn run_all(root: &Path) -> Vec<Finding> {
    let mut findings = boundaries::run(root);
    findings.extend(links::run(root));
    findings.extend(skills::run(root));
    findings.extend(prose::run(root));
    findings.extend(naming::run(root));
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

mod boundary {
    use super::*;

    #[test]
    fn bad_fixtures() {
        // A plain adapter dependency (name and path both offend) fires.
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "crates/slice/Cargo.toml",
            "[dependencies]\nvectis = { path = \"../../specify-adapters/targets/vectis\" }\n",
        );
        assert!(fired(&boundaries::run(dir.path()), boundaries::CHECK_ADAPTER_DEPENDENCY));

        // A renamed dependency is caught by its effective `package` name.
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "crates/slice/Cargo.toml",
            "[dev-dependencies]\nharmless = { package = \"captures\", version = \"1\" }\n",
        );
        assert!(fired(&boundaries::run(dir.path()), boundaries::CHECK_ADAPTER_DEPENDENCY));

        // The expanded `[dependencies.<name>]` table syntax fires.
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "harness/composed/Cargo.toml",
            "[dependencies.intent]\nversion = \"1\"\n",
        );
        assert!(fired(&boundaries::run(dir.path()), boundaries::CHECK_ADAPTER_DEPENDENCY));

        // A target-specific dependency table fires.
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "Cargo.toml", "[target.'cfg(unix)'.dependencies]\ntypescript = \"1\"\n");
        assert!(fired(&boundaries::run(dir.path()), boundaries::CHECK_ADAPTER_DEPENDENCY));

        // A source reaching into specify-adapters fires even when the name
        // collides with a legitimate crate (the omnia runtime vs target).
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "Cargo.toml",
            "[workspace.dependencies]\nomnia = { package = \"omnia\", path = \"../specify-adapters/targets/omnia\" }\n",
        );
        assert!(fired(&boundaries::run(dir.path()), boundaries::CHECK_ADAPTER_DEPENDENCY));

        // The legitimate omnia runtime dependency stays silent.
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "Cargo.toml",
            "[workspace.dependencies]\nomnia = \"0.35.0\"\nslice = { path = \"crates/slice\" }\n",
        );
        assert!(boundaries::run(dir.path()).is_empty());

        // Rust imports alone no longer fire: an undeclared crate cannot be
        // imported, so the manifest is the only enforcement point.
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "crates/slice/src/lib.rs", "use captures::operations;\n");
        assert!(boundaries::run(dir.path()).is_empty());
    }
}

// Each ported check must fire on a known-bad fixture; the matrices
// below cover one offending input per check.

mod links_matrix {
    use super::*;

    #[test]
    fn bad_fixtures() {
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
        write(
            dir.path(),
            "plugins/spec/skills/refine/SKILL.md",
            "---\nname: specify-refine\n---\n",
        );
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

        // A permanent surface linking into rfcs/ fires.
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "rfcs/roadmap.md", "# Roadmap\n");
        write(dir.path(), "docs/guide.md", "See [the roadmap](../rfcs/roadmap.md).\n");
        assert!(fired(&links::run(dir.path()), links::CHECK_RFCS_LINK));
    }
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

mod skills_matrix {
    use super::*;

    #[test]
    fn bad_fixtures() {
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

        // An orchestration/judgment heading in a spec skill body fires; the
        // same heading quoted inside a code fence, or in another plugin's
        // skill, stays silent.
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "plugins/spec/skills/refine/SKILL.md",
            &VALID_SKILL.replace("Body.", "## Synthesis playbook\n\nBody."),
        );
        assert!(fired(&skills::run(dir.path()), skills::CHECK_ORCHESTRATION_HEADING));
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "plugins/spec/skills/refine/SKILL.md",
            &VALID_SKILL.replace("Body.", "```md\n## Synthesis playbook\n```\n\nBody."),
        );
        assert!(!fired(&skills::run(dir.path()), skills::CHECK_ORCHESTRATION_HEADING));
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "plugins/capture/skills/wiretap/SKILL.md",
            "---\nname: capture-wiretap\ndescription: Run the wiretap capture. Use when capturing runtime traffic.\n---\n\n## Validation notes\n\nBody.\n",
        );
        assert!(!fired(&skills::run(dir.path()), skills::CHECK_ORCHESTRATION_HEADING));
    }
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
    write(root, "docs/standards/testing.md", "# Testing\n");
    write(root, "docs/contributing/quality-gates.md", "# Quality gates\n");
}

mod naming_matrix {
    use super::*;

    #[test]
    fn bad_fixtures() {
        // A test fn identifier over the cap fires; the module path is
        // context and does not count.
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "crates/demo/tests/area.rs",
            "#[test]\nfn a_very_long_test_name_that_exceeds_cap() {}\n",
        );
        assert!(fired(&naming::run(dir.path()), naming::CHECK_TEST_FN_LENGTH));

        // An async test behind further attributes fires too.
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "crates/demo/tests/area.rs",
            "#[tokio::test]\n#[ignore]\nasync fn a_very_long_test_name_that_exceeds_cap() {}\n",
        );
        assert!(fired(&naming::run(dir.path()), naming::CHECK_TEST_FN_LENGTH));

        // A name at the cap, and a long non-test fn, stay silent.
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "crates/demo/tests/area.rs",
            "#[test]\nfn exactly_thirty_characters_long() {}\n\nfn a_very_long_plain_helper_name_is_not_checked() {}\n",
        );
        assert!(naming::run(dir.path()).is_empty());
    }
}

mod prose_matrix {
    use super::*;

    #[test]
    fn bad_fixtures() {
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

        // A missing canonical review-team-protocol document fires.
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(fired(&prose::run(dir.path()), prose::CHECK_CANONICAL_MISSING));

        // A two-file reference corpus without a README.md index fires.
        let dir = tempfile::tempdir().expect("tempdir");
        write_valid_prose_tree(dir.path());
        write(dir.path(), "targets/demo/prose/references/providers/a.md", "A.\n");
        write(dir.path(), "targets/demo/prose/references/providers/b.md", "B.\n");
        assert!(fired(&prose::run(dir.path()), prose::CHECK_CORPUS_UNINDEXED));

        // A design-history citation fires; a standards RFC passes.
        let dir = tempfile::tempdir().expect("tempdir");
        write_valid_prose_tree(dir.path());
        write(dir.path(), "docs/explanation/why.md", "Per RFC-5 the loop was split.\n");
        assert!(fired(&prose::run(dir.path()), prose::CHECK_HISTORY_CITATION));
        let dir = tempfile::tempdir().expect("tempdir");
        write_valid_prose_tree(dir.path());
        write(dir.path(), "docs/explanation/time.md", "Timestamps follow RFC 3339.\n");
        assert!(!fired(&prose::run(dir.path()), prose::CHECK_HISTORY_CITATION));

        // A design-history citation in an engine code comment fires;
        // one in the rfcs/ tree stays out of scope.
        let dir = tempfile::tempdir().expect("tempdir");
        write_valid_prose_tree(dir.path());
        write(dir.path(), "wit/demo.wit", "/// Revised per RFC-61.\n");
        assert!(fired(&prose::run(dir.path()), prose::CHECK_HISTORY_CITATION));
        let dir = tempfile::tempdir().expect("tempdir");
        write_valid_prose_tree(dir.path());
        write(dir.path(), "rfcs/rfc-5-loop.md", "# RFC-5\n");
        assert!(!fired(&prose::run(dir.path()), prose::CHECK_HISTORY_CITATION));

        // A ```text fence with a flow arrow in an explanation doc fires.
        let dir = tempfile::tempdir().expect("tempdir");
        write_valid_prose_tree(dir.path());
        write(dir.path(), "docs/explanation/flow.md", "```text\nplan -> refine -> build\n```\n");
        assert!(fired(&prose::run(dir.path()), prose::CHECK_TEXT_DIAGRAM));
    }
}

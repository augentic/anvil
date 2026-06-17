//! Integration tests for `specify contract dump`.
//!
//! Locks the dump payload to `schemas/contract/dump.schema.json`,
//! pins sentinel rows from each contract section, and keeps the
//! `specify_error::codes::WIRE_CODES` registry honest with a source
//! scan over the workspace's production call sites.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use serde_json::Value;

mod common;
use common::{parse_stdout, repo_root, specify_cmd};

/// Dumped once per test process; tests share the parsed payload so the
/// binary is not re-spawned for every assertion suite.
static DUMP: LazyLock<Value> = LazyLock::new(|| {
    let assert = specify_cmd().args(["--format", "json", "contract", "dump"]).assert().success();
    parse_stdout(&assert.get_output().stdout, &repo_root())
});

fn dump_json() -> &'static Value {
    &DUMP
}

#[test]
fn dump_validates_against_schema() {
    let validator = specify_schema::compile_schema(specify_schema::CONTRACT_DUMP_JSON_SCHEMA)
        .expect("dump schema compiles");
    let dump = dump_json();
    let errors: Vec<String> = validator.iter_errors(dump).map(|err| err.to_string()).collect();
    assert!(errors.is_empty(), "contract dump must satisfy its schema; errors: {errors:?}");
}

#[test]
fn dump_carries_known_surface() {
    let dump = dump_json();
    assert_eq!(dump["version"], 1);
    assert_eq!(dump["binary-version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(dump["commands"]["name"], "specify");

    let top_verbs: BTreeSet<&str> = dump["commands"]["subcommands"]
        .as_array()
        .expect("subcommands array")
        .iter()
        .map(|node| node["name"].as_str().expect("verb name"))
        .collect();
    for verb in [
        "init",
        "source",
        "target",
        "rules",
        "extension",
        "lint",
        "slice",
        "archive",
        "plan",
        "journal",
        "registry",
        "workspace",
        "completions",
        "contract",
        "upgrade",
        "plugins",
    ] {
        assert!(top_verbs.contains(verb), "top-level verb `{verb}` missing: {top_verbs:?}");
    }

    let exit_codes: Vec<u64> = dump["exit-codes"]
        .as_array()
        .expect("exit-codes array")
        .iter()
        .map(|row| row["code"].as_u64().expect("numeric exit code"))
        .collect();
    assert_eq!(exit_codes, vec![0, 1, 2, 3]);

    let error_ids = dump["error-ids"].as_array().expect("error-ids array");
    assert!(error_ids.iter().any(|id| id == "adapter-not-found"));
    let event_ids = dump["journal-event-ids"].as_array().expect("event-ids array");
    assert!(event_ids.iter().any(|id| id == "slice.build.failed"));
    let schemas = dump["schemas"].as_array().expect("schemas array");
    assert!(schemas.iter().any(|p| p == "schemas/plan/plan.schema.json"));
    let tests = dump["tests"].as_array().expect("tests array");
    assert!(
        tests.iter().any(|p| p == "tests/cli_contract.rs"),
        "build-time tests inventory must carry this test file"
    );
}

#[test]
fn dump_text_format_summarises() {
    let assert = specify_cmd().args(["contract", "dump"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(
        stdout.starts_with(&format!("specify {}", env!("CARGO_PKG_VERSION"))),
        "text dump must lead with the binary version, got:\n{stdout}"
    );
    assert!(stdout.contains("error-ids:"), "text dump must summarise sections:\n{stdout}");
}

// -- WIRE_CODES registry parity ---------------------------------------

/// Collect every production `.rs` file under the workspace's `src/`
/// roots, skipping `tests/` directories and `tests.rs` modules.
fn production_sources() -> Vec<PathBuf> {
    let repo = repo_root();
    let mut roots = vec![repo.join("src")];
    let crates_dir = repo.join("crates");
    for entry in fs::read_dir(&crates_dir).expect("read crates/") {
        let entry = entry.expect("crates entry");
        let src = entry.path().join("src");
        if src.is_dir() {
            roots.push(src);
        }
    }
    let mut files = Vec::new();
    for root in roots {
        walk(&root, &mut files);
    }
    files
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap_or_else(|err| panic!("read {}: {err}", dir.display())) {
        let entry = entry.expect("dir entry");
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if path.is_dir() {
            if name == "tests" {
                continue;
            }
            walk(&path, out);
        } else if name.ends_with(".rs") && name != "tests.rs" {
            out.push(path);
        }
    }
}

/// Strip the conventional trailing `#[cfg(test)]` module so inline
/// test fixtures (e.g. `code: "kebab-prefix"`) do not count as
/// production call sites. House style keeps test modules at the end of
/// a file, so truncating at the first marker is sufficient.
fn production_slice(contents: &str) -> &str {
    contents.find("#[cfg(test)]").map_or(contents, |idx| &contents[..idx])
}

/// Every literal error code constructed in production sources must be
/// registered in `specify_error::codes::WIRE_CODES`, so the published
/// contract stays complete. Covers `Diag`/`Validation` struct literals
/// (`code: "…"`), `validation_failed("…")` call sites, and
/// `Filesystem { op: "…" }` composites (`filesystem-<op>`).
#[test]
fn wire_codes_cover_production_literals() {
    let code_literal = regex::Regex::new(r#"code: "([a-z0-9-]+)""#).expect("regex");
    let validation_call =
        regex::Regex::new(r#"validation_failed\(\s*"([a-z0-9-]+)""#).expect("regex");
    let filesystem_op = regex::Regex::new(r#"Filesystem \{\s*op: "([a-z0-9-]+)""#).expect("regex");

    let registry: BTreeSet<&str> = specify_error::codes::WIRE_CODES.iter().copied().collect();
    let mut missing: BTreeSet<String> = BTreeSet::new();

    for path in production_sources() {
        let contents = fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
        let body = production_slice(&contents);
        let mut require = |code: String| {
            if !registry.contains(code.as_str()) {
                missing.insert(code);
            }
        };
        for cap in code_literal.captures_iter(body) {
            require(cap[1].to_string());
        }
        for cap in validation_call.captures_iter(body) {
            require(cap[1].to_string());
        }
        for cap in filesystem_op.captures_iter(body) {
            require(format!("filesystem-{}", &cap[1]));
        }
    }

    assert!(
        missing.is_empty(),
        "error codes constructed in production sources but absent from \
         specify_error::codes::WIRE_CODES — add them to the registry:\n{missing:#?}"
    );
}

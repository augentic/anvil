//! Scenario-pack predicates: frontmatter schema, id uniqueness, body
//! id agreement, stage contiguity, artifact-path safety, and the
//! catalog↔runs drift check over `evals/`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::support::{Finding, frontmatter_split, rel, walk_files};

/// Scenario frontmatter fails `scenario.schema.json`.
pub const CHECK_SCHEMA_VIOLATION: &str = "scenarios.schema-violation";
/// Two scenario files declare the same frontmatter `id`.
pub const CHECK_DUPLICATE_ID: &str = "scenarios.duplicate-id";
/// A body `Scenario ID:` line disagrees with the frontmatter id.
pub const CHECK_BODY_ID_MISMATCH: &str = "scenarios.body-id-mismatch";
/// `stages` is not a contiguous slice of the slice-loop order.
pub const CHECK_STAGES_NOT_CONTIGUOUS: &str = "scenarios.stages-not-contiguous";
/// An `expected-artifacts` entry is empty, absolute, or escaping.
pub const CHECK_ARTIFACT_PATH_UNSAFE: &str = "scenarios.artifact-path-unsafe";
/// The catalog, scenario files, and run records disagree.
pub const CHECK_CATALOG_RUNS_DRIFT: &str = "scenarios.catalog-runs-drift";

/// The fixed slice-loop stage order a scenario's `stages` list must be
/// a contiguous slice of, anchored at any element.
const STAGES_ORDER: [&str; 5] = ["plan", "refine", "build", "merge", "drop"];

/// Catalog↔runs policy (formerly CORE-056's `config:`).
const CATALOG: &str = "evals/scenarios/README.md";
const SCENARIOS_DIR: &str = "evals/scenarios";
const RUNS_DIR: &str = "evals/runs";
const STATUSES: &[&str] = &["pending", "parked", "passed", "failed", "deferred"];
const GATES: &[&str] = &["release-blocker", "full"];
const STATUS_RESULT_MAP: &[(&str, &str)] =
    &[("passed", "pass"), ("failed", "fail"), ("deferred", "deferred")];

/// Run every scenario predicate rooted at `root`.
pub fn run(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    let opted = collect_opted_scenarios(root);
    check_schema(&opted, &mut findings);
    check_duplicate_ids(&opted, &mut findings);
    check_body_id(&opted, &mut findings);
    check_stages(&opted, &mut findings);
    check_artifact_paths(&opted, &mut findings);
    check_catalog_runs(root, &mut findings);
    findings
}

/// An opted-in scenario file (leading `---` frontmatter block) with
/// its parsed frontmatter; YAML that fails to parse yields an empty
/// map so the schema check still flags the file.
struct ScenarioFile {
    rel: String,
    body: String,
    frontmatter: JsonMap<String, JsonValue>,
}

/// Discover scenario candidates: flat `evals/scenarios/<id>.md` files
/// (skipping the catalog `README.md`), `targets/<a>/tests/…` scenario
/// files, and `plugins/<p>/skills/<s>/fixtures/<case>/scenario.md`.
fn discover_candidates(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_evals(&root.join("evals").join("scenarios"), &mut out);
    collect_targets(root, &mut out);
    collect_plugin_fixtures(&root.join("plugins"), &mut out);
    out.sort();
    out.dedup();
    out
}

fn collect_evals(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let is_md = path.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"));
        if name != "README.md" && is_md {
            out.push(path);
        }
    }
}

/// `targets/<adapter>/tests/<file>.md` and `…/tests/<dir>/scenario.md`
/// under either the flattened or nested (`adapters/`) root shape.
fn collect_targets(root: &Path, out: &mut Vec<PathBuf>) {
    let nested = root.join("adapters").join("targets");
    let targets_dir = if nested.is_dir() { nested } else { root.join("targets") };
    let mut files = Vec::new();
    walk_files(&targets_dir, &mut files);
    for path in files {
        let Ok(relative) = path.strip_prefix(&targets_dir) else {
            continue;
        };
        let parts: Vec<&str> = relative.iter().filter_map(|c| c.to_str()).collect();
        let is_md = path.extension().and_then(|e| e.to_str()) == Some("md");
        if is_md && parts.len() == 3 && parts[1] == "tests" {
            out.push(path.clone());
        }
        if parts.len() == 4 && parts[1] == "tests" && parts[3] == "scenario.md" {
            out.push(path);
        }
    }
}

fn collect_plugin_fixtures(plugins_dir: &Path, out: &mut Vec<PathBuf>) {
    let mut files = Vec::new();
    walk_files(plugins_dir, &mut files);
    for path in files {
        if path.file_name().and_then(|n| n.to_str()) != Some("scenario.md") {
            continue;
        }
        let Ok(relative) = path.strip_prefix(plugins_dir) else {
            continue;
        };
        let parts: Vec<&str> = relative.iter().filter_map(|c| c.to_str()).collect();
        if parts.len() == 6
            && parts[1] == "skills"
            && parts[3] == "fixtures"
            && parts[5] == "scenario.md"
        {
            out.push(path);
        }
    }
}

fn collect_opted_scenarios(root: &Path) -> Vec<ScenarioFile> {
    let mut opted = Vec::new();
    for path in discover_candidates(root) {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let Some((block, body)) = frontmatter_split(&content) else {
            continue;
        };
        let frontmatter = match serde_saphyr::from_str::<JsonValue>(block) {
            Ok(JsonValue::Object(map)) => map,
            Ok(_) | Err(_) => JsonMap::new(),
        };
        opted.push(ScenarioFile {
            rel: rel(root, &path),
            body: body.to_owned(),
            frontmatter,
        });
    }
    opted
}

/// Every opted-in scenario's frontmatter must satisfy the embedded
/// `scenario.schema.json` (an unparseable block validates as empty and
/// fails the schema's required fields).
fn check_schema(opted: &[ScenarioFile], findings: &mut Vec<Finding>) {
    let validator = match schema::cached_validator(schema::SCENARIO_JSON_SCHEMA) {
        Ok(validator) => validator,
        Err(error) => {
            findings.push(Finding::new(
                CHECK_SCHEMA_VIOLATION,
                format!("cannot compile scenario.schema.json: {error}"),
            ));
            return;
        }
    };
    for sc in opted {
        let value = JsonValue::Object(sc.frontmatter.clone());
        for error in validator.iter_errors(&value) {
            findings.push(Finding::new(
                CHECK_SCHEMA_VIOLATION,
                format!(
                    "{} — frontmatter violates scenario.schema.json at {}: {error}",
                    sc.rel,
                    error.instance_path()
                ),
            ));
        }
    }
}

/// Each frontmatter `id` must be unique across the discovered set.
fn check_duplicate_ids(opted: &[ScenarioFile], findings: &mut Vec<Finding>) {
    let mut by_id: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for sc in opted {
        if let Some(JsonValue::String(id)) = sc.frontmatter.get("id") {
            by_id.entry(id.as_str()).or_default().push(sc.rel.as_str());
        }
    }
    for (id, paths) in by_id {
        if paths.len() > 1 {
            findings.push(Finding::new(
                CHECK_DUPLICATE_ID,
                format!("scenario id '{id}' declared by more than one file: {paths:?}"),
            ));
        }
    }
}

static SCENARIO_ID_BODY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^Scenario ID:\s*`?([a-z][a-z0-9-]*)`?\s*$").expect("scenario id pattern")
});

/// The body `Scenario ID:` line must agree with the frontmatter id.
fn check_body_id(opted: &[ScenarioFile], findings: &mut Vec<Finding>) {
    for sc in opted {
        let Some(JsonValue::String(id)) = sc.frontmatter.get("id") else {
            continue;
        };
        let Some(caps) = SCENARIO_ID_BODY_RE.captures(&sc.body) else {
            continue;
        };
        let body_id = caps.get(1).expect("capture group").as_str();
        if body_id != id {
            findings.push(Finding::new(
                CHECK_BODY_ID_MISMATCH,
                format!(
                    "{} — body 'Scenario ID: `{body_id}`' does not match frontmatter id '{id}'",
                    sc.rel
                ),
            ));
        }
    }
}

/// Each non-empty frontmatter's `stages` must be a contiguous slice of
/// the slice-loop order anchored at any element.
fn check_stages(opted: &[ScenarioFile], findings: &mut Vec<Finding>) {
    for sc in opted {
        if sc.frontmatter.is_empty() {
            continue;
        }
        if let Some(stages) = sc.frontmatter.get("stages")
            && !is_contiguous_stages(stages)
        {
            findings.push(Finding::new(
                CHECK_STAGES_NOT_CONTIGUOUS,
                format!(
                    "{} — stages must be a contiguous slice of [plan, refine, build, merge, \
                     drop]; got {}",
                    sc.rel,
                    serde_json::to_string(stages).unwrap_or_else(|_| "<?>".into())
                ),
            ));
        }
    }
}

fn is_contiguous_stages(stages: &JsonValue) -> bool {
    let Some(stages) = stages.as_array() else {
        return false;
    };
    if stages.is_empty() {
        return false;
    }
    let first = stages[0].as_str().unwrap_or("");
    let Some(start) = STAGES_ORDER.iter().position(|s| *s == first) else {
        return false;
    };
    stages.iter().enumerate().all(|(i, stage)| {
        start + i < STAGES_ORDER.len() && stage.as_str() == Some(STAGES_ORDER[start + i])
    })
}

/// Every `expected-artifacts` entry must be a non-empty, relative,
/// non-escaping path.
fn check_artifact_paths(opted: &[ScenarioFile], findings: &mut Vec<Finding>) {
    for sc in opted {
        let Some(JsonValue::Array(artifacts)) = sc.frontmatter.get("expected-artifacts") else {
            continue;
        };
        for artifact in artifacts {
            let Some(a) = artifact.as_str() else {
                continue;
            };
            let detail = if a.is_empty() {
                "expected-artifacts entry is empty".to_owned()
            } else if a.starts_with('/') {
                format!("expected-artifact '{a}' must be relative, not absolute")
            } else if a.split('/').any(|segment| segment == "..") {
                format!("expected-artifact '{a}' must not escape the scenario workspace")
            } else {
                continue;
            };
            findings
                .push(Finding::new(CHECK_ARTIFACT_PATH_UNSAFE, format!("{} — {detail}", sc.rel)));
        }
    }
}

/// One parsed catalog table row: the id from the File link plus the
/// Status / Gate cells.
struct CatalogRow {
    id: String,
    status: String,
    gate: Option<String>,
}

static ROW_LINK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\[`?([a-z][a-z0-9-]*)`?\]\(([^)]+)\)$").expect("catalog row pattern")
});

/// The scenario catalog, the scenario files, and the committed run
/// records must agree (formerly CORE-056).
fn check_catalog_runs(root: &Path, findings: &mut Vec<Finding>) {
    let Ok(content) = fs::read_to_string(root.join(CATALOG)) else {
        findings.push(Finding::new(
            CHECK_CATALOG_RUNS_DRIFT,
            format!("catalog file {CATALOG} cannot be read"),
        ));
        return;
    };
    let rows = parse_rows(&content, findings);
    check_row_values(&rows, findings);
    check_file_parity(root, &rows, findings);
    let records = collect_records(root, findings);
    check_record_agreement(&rows, &records, findings);
}

fn drift(detail: &str) -> Finding {
    Finding::new(CHECK_CATALOG_RUNS_DRIFT, format!("{CATALOG} — {detail}"))
}

fn parse_rows(content: &str, findings: &mut Vec<Finding>) -> Vec<CatalogRow> {
    let mut rows = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('|') {
            continue;
        }
        let cells: Vec<&str> = trimmed.trim_matches('|').split('|').map(str::trim).collect();
        let Some(file_cell) = cells.get(1) else {
            continue;
        };
        if !file_cell.contains("](") {
            continue;
        }
        let Some(caps) = ROW_LINK_RE.captures(file_cell) else {
            findings.push(drift(&format!(
                "table row File cell '{file_cell}' does not parse as [`<id>`](<id>.md)"
            )));
            continue;
        };
        let label = caps[1].to_owned();
        let target = caps[2].to_owned();
        let id = target.strip_suffix(".md").unwrap_or(&target).to_owned();
        if label != id {
            findings
                .push(drift(&format!("row label '{label}' disagrees with linked file '{target}'")));
        }
        rows.push(CatalogRow {
            id,
            status: cells.get(2).copied().unwrap_or_default().to_owned(),
            gate: cells.get(3).map(|cell| (*cell).to_owned()),
        });
    }
    rows
}

fn check_row_values(rows: &[CatalogRow], findings: &mut Vec<Finding>) {
    let mut seen: BTreeMap<&str, u32> = BTreeMap::new();
    for row in rows {
        *seen.entry(row.id.as_str()).or_default() += 1;
        if !STATUSES.contains(&row.status.as_str()) {
            findings.push(drift(&format!(
                "row '{}' status '{}' is not one of [{}]",
                row.id,
                row.status,
                STATUSES.join(", ")
            )));
        }
        match &row.gate {
            None => findings.push(drift(&format!("row '{}' is missing the Gate column", row.id))),
            Some(gate) if !GATES.contains(&gate.as_str()) => {
                findings.push(drift(&format!(
                    "row '{}' gate '{gate}' is not one of [{}]",
                    row.id,
                    GATES.join(", ")
                )));
            }
            Some(_) => {}
        }
    }
    for (id, count) in seen {
        if count > 1 {
            findings.push(drift(&format!("duplicate catalog rows for '{id}'")));
        }
    }
}

fn check_file_parity(root: &Path, rows: &[CatalogRow], findings: &mut Vec<Finding>) {
    let file_ids = markdown_stems(&root.join(SCENARIOS_DIR));
    let row_ids: BTreeSet<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    for row in rows {
        if !file_ids.contains(&row.id) {
            findings.push(drift(&format!(
                "row '{id}' has no scenario file {SCENARIOS_DIR}/{id}.md",
                id = row.id
            )));
        }
    }
    for file_id in &file_ids {
        if !row_ids.contains(file_id.as_str()) {
            findings.push(Finding::new(
                CHECK_CATALOG_RUNS_DRIFT,
                format!("scenario file {SCENARIOS_DIR}/{file_id}.md has no catalog row"),
            ));
        }
    }
}

fn markdown_stems(dir: &Path) -> BTreeSet<String> {
    let mut stems = BTreeSet::new();
    let Ok(entries) = fs::read_dir(dir) else {
        return stems;
    };
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name == "README.md" {
            continue;
        }
        if let Some(stem) = name.strip_suffix(".md") {
            stems.insert(stem.to_owned());
        }
    }
    stems
}

/// One committed run record parsed from its `<id>.<result>.md` name.
struct RunRecord {
    id: String,
    result: String,
    rel: String,
}

fn collect_records(root: &Path, findings: &mut Vec<Finding>) -> Vec<RunRecord> {
    let legal_results: BTreeSet<&str> = STATUS_RESULT_MAP.iter().map(|(_, r)| *r).collect();
    let mut records = Vec::new();
    for stem in markdown_stems(&root.join(RUNS_DIR)) {
        let record_rel = format!("{RUNS_DIR}/{stem}.md");
        let Some((id, result)) = stem.rsplit_once('.') else {
            findings.push(Finding::new(
                CHECK_CATALOG_RUNS_DRIFT,
                format!("{record_rel} — run record filename must be <id>.<result>.md"),
            ));
            continue;
        };
        if !legal_results.contains(result) {
            findings.push(Finding::new(
                CHECK_CATALOG_RUNS_DRIFT,
                format!(
                    "{record_rel} — record result '{result}' is not one of [{}]",
                    legal_results.iter().copied().collect::<Vec<_>>().join(", ")
                ),
            ));
            continue;
        }
        records.push(RunRecord {
            id: id.to_owned(),
            result: result.to_owned(),
            rel: record_rel,
        });
    }
    records
}

fn check_record_agreement(rows: &[CatalogRow], records: &[RunRecord], findings: &mut Vec<Finding>) {
    let mut by_id: BTreeMap<&str, Vec<&RunRecord>> = BTreeMap::new();
    for record in records {
        by_id.entry(record.id.as_str()).or_default().push(record);
    }
    for (id, group) in &by_id {
        if group.len() > 1 {
            findings.push(drift(&format!(
                "multiple run records for '{id}': {}",
                group.iter().map(|r| r.rel.as_str()).collect::<Vec<_>>().join(", ")
            )));
        }
    }
    let row_ids: BTreeSet<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    for record in records {
        if !row_ids.contains(record.id.as_str()) {
            findings.push(Finding::new(
                CHECK_CATALOG_RUNS_DRIFT,
                format!(
                    "{} — record names scenario '{}' which has no catalog row",
                    record.rel, record.id
                ),
            ));
        }
    }
    for row in rows {
        if !STATUSES.contains(&row.status.as_str()) {
            continue;
        }
        let group = by_id.get(row.id.as_str());
        let expected = STATUS_RESULT_MAP
            .iter()
            .find(|(status, _)| *status == row.status)
            .map(|(_, result)| *result);
        match expected {
            Some(expected) => {
                let satisfied =
                    group.is_some_and(|records| records.iter().any(|r| r.result == expected));
                if !satisfied {
                    findings.push(drift(&format!(
                        "row '{id}' status '{status}' requires committed record \
                         {RUNS_DIR}/{id}.{expected}.md",
                        id = row.id,
                        status = row.status
                    )));
                }
            }
            None => {
                for record in group.into_iter().flatten() {
                    findings.push(drift(&format!(
                        "record {} disagrees with the '{}' row for '{}'",
                        record.rel, row.status, row.id
                    )));
                }
            }
        }
    }
}

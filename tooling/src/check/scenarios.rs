use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use serde_json::Value as JsonValue;
use walkdir::WalkDir;

use crate::context::Context;
use crate::finding::{Check, Finding, Location};
use crate::helpers::{relative_display, under_symlink};
use crate::schema::{collect_errors_for_test, SchemaId};

pub const RULE_SCHEMA_VIOLATION: &str = "scenarios.schema-violation";
pub const RULE_STAGES_NOT_CONTIGUOUS: &str = "scenarios.stages-not-contiguous-prefix";
pub const RULE_BODY_ID_MISMATCH: &str = "scenarios.body-id-mismatch";
pub const RULE_ARTIFACT_PATH_UNSAFE: &str = "scenarios.artifact-path-unsafe";
pub const RULE_DUPLICATE_ID: &str = "scenarios.duplicate-id";
pub const RULE_RECORDED_TRACE_VIOLATION: &str = "scenarios.recorded-trace-violation";
pub const RULE_STALE_RECORDED_TRACE: &str = "scenarios.stale-recorded-trace";

const STAGES_ORDER: [&str; 5] = ["plan", "refine", "build", "merge", "drop"];

const TRACE_REQUIRED_FIELDS: [&str; 6] = [
    "kind",
    "schemaVersion",
    "sourceBackend",
    "sourceRunId",
    "sourceTimestamp",
    "scenarioId",
];

/// Scenario frontmatter validation and recorded-trace freshness checks.
pub struct ScenariosCheck;

impl Check for ScenariosCheck {
    fn id(&self) -> &'static str {
        "scenarios"
    }

    fn run(&self, ctx: &Context) -> Vec<Finding> {
        let mut findings = validate_scenario_frontmatter(ctx);
        findings.extend(check_recorded_trace_freshness(ctx));
        findings
    }
}

struct ScenarioFile {
    path: PathBuf,
    rel: String,
    content: String,
    frontmatter: BTreeMap<String, JsonValue>,
}

/// Run scenario frontmatter validation only (tests / direct invocation).
pub fn validate_scenario_frontmatter(ctx: &Context) -> Vec<Finding> {
    let validator = match ctx.schema(
        ctx.framework_schema_dir()
            .join(SchemaId::Scenario.file_name()),
    ) {
        Ok(v) => v,
        Err(error) => {
            return vec![finding(
                RULE_SCHEMA_VIOLATION,
                format!("Scenario frontmatter: cannot load scenario schema: {error}"),
                None,
            )];
        }
    };

    let candidate_paths = discover_scenario_candidates(ctx);
    let mut opted = Vec::new();
    let mut findings = Vec::new();

    for path in candidate_paths {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let rel = relative_display(ctx.framework_root(), &path);
        let Some(fm_block) = frontmatter_block(&content) else {
            continue;
        };

        let fm = match parse_frontmatter_yaml(fm_block) {
            Ok(fm) => fm,
            Err(msg) => {
                findings.push(finding(
                    RULE_SCHEMA_VIOLATION,
                    format!("Scenario frontmatter: {rel} — invalid YAML: {msg}"),
                    Some(path),
                ));
                continue;
            }
        };

        opted.push(ScenarioFile {
            path,
            rel,
            content,
            frontmatter: fm,
        });
    }

    for sc in &opted {
        let value = JsonValue::Object(sc.frontmatter.clone().into_iter().collect());
        if let Err(errors) = collect_errors_for_test(&validator, &value) {
            for error in errors {
                let at = if error.instance_path.is_empty() {
                    "/".to_string()
                } else {
                    error.instance_path
                };
                findings.push(finding(
                    RULE_SCHEMA_VIOLATION,
                    format!(
                        "Scenario frontmatter: {} — {} {}",
                        sc.rel, at, error.message
                    )
                    .trim()
                    .to_string(),
                    Some(sc.path.clone()),
                ));
            }
        }
    }

    for sc in &opted {
        if sc.frontmatter.is_empty() {
            continue;
        }
        if let Some(stages) = sc.frontmatter.get("stages") {
            if !is_contiguous_stages_prefix(stages) {
                findings.push(finding(
                    RULE_STAGES_NOT_CONTIGUOUS,
                    format!(
                        "Scenario frontmatter: {} — stages must be a contiguous slice of \
                         [plan, refine, build, merge, drop] anchored at any element; got {}",
                        sc.rel,
                        serde_json::to_string(stages).unwrap_or_else(|_| "<?>".into())
                    ),
                    Some(sc.path.clone()),
                ));
            }
        }
    }

    let scenario_id_body_re =
        Regex::new(r"(?m)^Scenario ID:\s*`?([a-z][a-z0-9-]*)`?\s*$").expect("valid regex");

    for sc in &opted {
        let Some(JsonValue::String(id)) = sc.frontmatter.get("id") else {
            continue;
        };
        let body = body_after_frontmatter(&sc.content);
        let Some(caps) = scenario_id_body_re.captures(&body) else {
            continue;
        };
        let body_id = caps.get(1).expect("capture group").as_str();
        if body_id != id {
            findings.push(finding(
                RULE_BODY_ID_MISMATCH,
                format!(
                    "Scenario frontmatter: {} — body 'Scenario ID: `{body_id}`' does not match \
                     frontmatter id '{id}'; align the visible line with the frontmatter id",
                    sc.rel
                ),
                Some(sc.path.clone()),
            ));
        }
    }

    for sc in &opted {
        let Some(JsonValue::Array(arts)) = sc.frontmatter.get("expected-artifacts") else {
            continue;
        };
        for art in arts {
            let Some(a) = art.as_str() else {
                continue;
            };
            if a.is_empty() {
                findings.push(finding(
                    RULE_ARTIFACT_PATH_UNSAFE,
                    format!(
                        "Scenario frontmatter: {} — expected-artifacts entry is empty",
                        sc.rel
                    ),
                    Some(sc.path.clone()),
                ));
                continue;
            }
            if a.starts_with('/') {
                findings.push(finding(
                    RULE_ARTIFACT_PATH_UNSAFE,
                    format!(
                        "Scenario frontmatter: {} — expected-artifact '{a}' must be relative to \
                         the scenario workspace, not absolute",
                        sc.rel
                    ),
                    Some(sc.path.clone()),
                ));
                continue;
            }
            if a.split('/').any(|seg| seg == "..") {
                findings.push(finding(
                    RULE_ARTIFACT_PATH_UNSAFE,
                    format!(
                        "Scenario frontmatter: {} — expected-artifact '{a}' must not escape the \
                         scenario workspace ('..' segment not allowed)",
                        sc.rel
                    ),
                    Some(sc.path.clone()),
                ));
            }
        }
    }

    let mut ids_by_value: HashMap<String, Vec<String>> = HashMap::new();
    for sc in &opted {
        let Some(JsonValue::String(id)) = sc.frontmatter.get("id") else {
            continue;
        };
        ids_by_value
            .entry(id.clone())
            .or_default()
            .push(sc.rel.clone());
    }
    for (id, paths) in ids_by_value {
        if paths.len() > 1 {
            findings.push(finding(
                RULE_DUPLICATE_ID,
                format!(
                    "Scenario frontmatter: duplicate scenario id '{id}' across files: {}",
                    paths.join(", ")
                ),
                None,
            ));
        }
    }

    findings.sort_by(|a, b| a.message.cmp(&b.message));
    findings
}

/// Run recorded-trace header validation and best-effort recency hints.
pub fn check_recorded_trace_freshness(ctx: &Context) -> Vec<Finding> {
    let recorded_root = ctx.framework_root().join("tests").join("recorded");
    if !recorded_root.is_dir() {
        return Vec::new();
    }

    let mut trace_paths = Vec::new();
    for entry in WalkDir::new(&recorded_root)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
            continue;
        }
        if under_symlink(ctx.framework_root(), &path)
            .unwrap_or(true)
        {
            continue;
        }
        trace_paths.push(path);
    }
    trace_paths.sort();

    let mut findings = Vec::new();
    let mut headers_by_path: HashMap<PathBuf, JsonValue> = HashMap::new();

    for path in &trace_paths {
        let rel = relative_display(ctx.framework_root(), path);
        let content = match fs::read_to_string(path) {
            Ok(content) => content,
            Err(source) => {
                findings.push(finding(
                    RULE_RECORDED_TRACE_VIOLATION,
                    format!("Recorded trace: {rel} — cannot read: {source}"),
                    Some(path.clone()),
                ));
                continue;
            }
        };

        let first_line = content.lines().next().unwrap_or("").trim();
        if first_line.is_empty() {
            findings.push(finding(
                RULE_RECORDED_TRACE_VIOLATION,
                format!(
                    "Recorded trace: {rel} — empty file (expected a 'recorded-trace-header' line first)"
                ),
                Some(path.clone()),
            ));
            continue;
        }

        let parsed: JsonValue = match serde_json::from_str(first_line) {
            Ok(value) => value,
            Err(source) => {
                findings.push(finding(
                    RULE_RECORDED_TRACE_VIOLATION,
                    format!("Recorded trace: {rel} — first line is not valid JSON: {source}"),
                    Some(path.clone()),
                ));
                continue;
            }
        };

        if !parsed.is_object() {
            findings.push(finding(
                RULE_RECORDED_TRACE_VIOLATION,
                format!("Recorded trace: {rel} — first line must be a JSON object"),
                Some(path.clone()),
            ));
            continue;
        }

        let header = parsed.clone();
        let kind = header.get("kind").and_then(JsonValue::as_str);
        if kind != Some("recorded-trace-header") {
            findings.push(finding(
                RULE_RECORDED_TRACE_VIOLATION,
                format!(
                    "Recorded trace: {rel} — first line kind must be 'recorded-trace-header' (got {})",
                    serde_json::to_string(header.get("kind").unwrap_or(&JsonValue::Null))
                        .unwrap_or_else(|_| "<unknown>".into())
                ),
                Some(path.clone()),
            ));
            continue;
        }

        let schema_version = header.get("schemaVersion");
        if schema_version != Some(&JsonValue::Number(1.into())) {
            findings.push(finding(
                RULE_RECORDED_TRACE_VIOLATION,
                format!(
                    "Recorded trace: {rel} — recorded-trace-header.schemaVersion must be 1 (got {})",
                    serde_json::to_string(schema_version.unwrap_or(&JsonValue::Null))
                        .unwrap_or_else(|_| "<unknown>".into())
                ),
                Some(path.clone()),
            ));
        }

        for field in TRACE_REQUIRED_FIELDS {
            let value = header.get(field);
            let missing = match value {
                None | Some(JsonValue::Null) => true,
                Some(JsonValue::String(s)) => s.is_empty(),
                _ => false,
            };
            if missing {
                findings.push(finding(
                    RULE_RECORDED_TRACE_VIOLATION,
                    format!(
                        "Recorded trace: {rel} — recorded-trace-header missing required field '{field}'"
                    ),
                    Some(path.clone()),
                ));
            }
        }

        headers_by_path.insert(path.clone(), header);
    }

    emit_stale_trace_hints(ctx, &trace_paths, &headers_by_path);

    findings.sort_by(|a, b| a.message.cmp(&b.message));
    findings
}

fn emit_stale_trace_hints(
    ctx: &Context,
    trace_paths: &[PathBuf],
    headers_by_path: &HashMap<PathBuf, JsonValue>,
) {
    let output = Command::new("git")
        .args(["diff", "--name-only", "HEAD~1..HEAD"])
        .current_dir(ctx.framework_root())
        .output();

    let Ok(output) = output else {
        return;
    };
    if !output.status.success() {
        return;
    }

    let diff: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect();

    let traces_by_rel: HashMap<String, &PathBuf> = trace_paths
        .iter()
        .map(|path| (relative_display(ctx.framework_root(), path), path))
        .collect();

    for rel in diff {
        let Some(path) = traces_by_rel.get(&rel) else {
            continue;
        };
        let header = headers_by_path.get(*path);
        let run_id = header
            .and_then(|h| h.get("sourceRunId"))
            .and_then(JsonValue::as_str)
            .unwrap_or("<unknown>");
        let ts = header
            .and_then(|h| h.get("sourceTimestamp"))
            .and_then(JsonValue::as_str)
            .unwrap_or("<unknown>");
        eprintln!(
            "WARN: {RULE_STALE_RECORDED_TRACE}: Recorded trace updated in HEAD: {rel} — \
             consider quoting sourceRunId='{run_id}' / sourceTimestamp='{ts}' in the commit \
             message so reviewers can trace it back to the live run."
        );
    }
}

fn discover_scenario_candidates(ctx: &Context) -> Vec<PathBuf> {
    let root = ctx.framework_root();
    let mut candidates = Vec::new();

    collect_tests_suite_scenarios(&root.join("tests"), 2, &mut candidates);
    collect_tests_suite_scenarios(&root.join("tests").join("suites"), 2, &mut candidates);
    collect_plan_scenarios(&root.join("tests").join("plan"), &mut candidates);
    collect_target_scenarios(&ctx.targets_dir(), &mut candidates);
    collect_plugin_fixture_scenarios(&root.join("plugins"), root, &mut candidates);

    candidates.sort();
    candidates.dedup();
    candidates
}

fn collect_tests_suite_scenarios(dir: &Path, max_depth: usize, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    for entry in WalkDir::new(dir)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if path.file_name().and_then(|name| name.to_str()) != Some("scenario.md") {
            continue;
        }
        let rel = path.strip_prefix(dir).unwrap_or(&path);
        let parts: Vec<_> = rel.components().collect();
        if parts.len() == 2 {
            out.push(path);
        }
    }
}

fn collect_plan_scenarios(dir: &Path, out: &mut Vec<PathBuf>) {
    if !dir.is_dir() {
        return;
    }
    for entry in WalkDir::new(dir)
        .max_depth(1)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let rel = path.strip_prefix(dir).unwrap_or(&path);
        if rel.components().count() == 1 {
            out.push(path);
        }
    }
}

fn collect_target_scenarios(targets_dir: &Path, out: &mut Vec<PathBuf>) {
    if !targets_dir.is_dir() {
        return;
    }
    for entry in WalkDir::new(targets_dir)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        let rel = path.strip_prefix(targets_dir).unwrap_or(&path);
        let parts: Vec<_> = rel
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();
        if parts.len() == 3 && parts[1] == "tests" {
            out.push(path.clone());
        }
        if parts.len() == 4 && parts[1] == "tests" && parts[3] == "scenario.md" {
            out.push(path);
        }
    }
}

fn collect_plugin_fixture_scenarios(plugins_dir: &Path, root: &Path, out: &mut Vec<PathBuf>) {
    if !plugins_dir.is_dir() {
        return;
    }
    for entry in WalkDir::new(plugins_dir)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if path.file_name().and_then(|name| name.to_str()) != Some("scenario.md") {
            continue;
        }
        if under_symlink(root, &path).unwrap_or(true) {
            continue;
        }
        let rel = path.strip_prefix(plugins_dir).unwrap_or(&path);
        let parts: Vec<_> = rel
            .components()
            .filter_map(|c| c.as_os_str().to_str())
            .collect();
        if parts.len() == 6
            && parts[1] == "skills"
            && parts[3] == "fixtures"
            && parts[5] == "scenario.md"
        {
            out.push(path);
        }
    }
}

fn is_contiguous_stages_prefix(stages: &JsonValue) -> bool {
    let Some(stages) = stages.as_array() else {
        return false;
    };
    if stages.is_empty() {
        return false;
    }
    let first = stages[0].as_str().unwrap_or("");
    let start = STAGES_ORDER.iter().position(|s| *s == first);
    let Some(start) = start else {
        return false;
    };
    for (i, stage) in stages.iter().enumerate() {
        if start + i >= STAGES_ORDER.len() {
            return false;
        }
        if stage.as_str() != Some(STAGES_ORDER[start + i]) {
            return false;
        }
    }
    true
}

fn frontmatter_block(content: &str) -> Option<&str> {
    let rest = content.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

fn parse_frontmatter_yaml(body: &str) -> Result<BTreeMap<String, JsonValue>, String> {
    serde_saphyr::from_str(body).map_err(|source| source.to_string())
}

fn body_after_frontmatter(content: &str) -> String {
    let Some(block) = content.strip_prefix("---\n") else {
        return content.to_string();
    };
    let Some(end) = block.find("\n---") else {
        return content.to_string();
    };
    let consumed = "---\n".len() + end + "\n---".len();
    content[consumed..].to_string()
}

fn finding(rule_id: &'static str, message: String, path: Option<PathBuf>) -> Finding {
    Finding {
        rule_id,
        message,
        location: path.map(|path| Location {
            path,
            line: 1,
            column: None,
        }),
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn contiguous_stages_prefix_accepts_refine_anchor() {
        assert!(is_contiguous_stages_prefix(&json!(["refine", "build"])));
    }

    #[test]
    fn contiguous_stages_prefix_rejects_gap() {
        assert!(!is_contiguous_stages_prefix(&json!(["plan", "build"])));
    }

    #[test]
    fn contiguous_stages_prefix_rejects_unknown_stage() {
        assert!(!is_contiguous_stages_prefix(&json!(["draft"])));
    }
}

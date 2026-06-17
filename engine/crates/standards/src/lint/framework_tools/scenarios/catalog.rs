//! Catalog↔runs drift check (CORE-056): the scenario catalog's group
//! tables, the scenario files on disk, and the committed run records
//! must agree. All policy — paths, legal value sets, the status↔result
//! agreement map — arrives in the rule's forwarded `config:`; nothing
//! here is rule-specific.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use regex::Regex;
use serde_json::Value as JsonValue;

use super::ScenarioFinding;

/// Codex id stamped on every catalog↔runs drift finding.
pub const RULE_CATALOG_RUNS_DRIFT: &str = "CORE-056";

/// CORE-056 policy parsed from the rule's forwarded `config:`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
struct CatalogPolicy {
    /// Project-relative path of the catalog markdown (the group tables).
    catalog: String,
    /// Project-relative directory holding one `<id>.md` per scenario.
    scenarios_dir: String,
    /// Project-relative directory holding `<id>.<result>.md` run records.
    runs_dir: String,
    /// Legal `Status` column values.
    statuses: Vec<String>,
    /// Legal `Gate` column values; an empty list skips gate validation.
    gates: Vec<String>,
    /// Status → record `<result>` token for statuses that require a
    /// committed record; statuses absent from the map must have none.
    status_result_map: BTreeMap<String, String>,
}

impl CatalogPolicy {
    /// Parse the forwarded rule `config:`. `None` — or a config object
    /// without a `catalog` key — disables the check; a config that
    /// declares `catalog` but fails to parse is an `Err` the caller
    /// surfaces as a finding.
    fn parse(config: Option<&JsonValue>) -> Result<Option<Self>, String> {
        let Some(value) = config else {
            return Ok(None);
        };
        if value.get("catalog").is_none() {
            return Ok(None);
        }
        serde_json::from_value(value.clone()).map(Some).map_err(|err| err.to_string())
    }
}

/// Resolve the forwarded `config:` into catalog↔runs findings: absent
/// config is a no-op, a malformed config is itself a finding, a parsed
/// policy runs the check.
pub fn findings_from_config(
    project_dir: &Path, config: Option<&JsonValue>,
) -> Vec<ScenarioFinding> {
    match CatalogPolicy::parse(config) {
        Ok(None) => Vec::new(),
        Ok(Some(policy)) => check_catalog_runs(project_dir, &policy),
        Err(detail) => vec![ScenarioFinding {
            rule_id: RULE_CATALOG_RUNS_DRIFT,
            path: None,
            message: format!("Scenario catalog: invalid catalog-runs config — {detail}"),
        }],
    }
}

/// Run the catalog↔runs drift check rooted at `project_dir` under the
/// given policy.
fn check_catalog_runs(project_dir: &Path, policy: &CatalogPolicy) -> Vec<ScenarioFinding> {
    let catalog_rel = policy.catalog.as_str();
    let Ok(content) = std::fs::read_to_string(project_dir.join(catalog_rel)) else {
        return vec![drift(catalog_rel, &format!("catalog file {catalog_rel} cannot be read"))];
    };
    let mut findings = Vec::new();
    let rows = parse_rows(&content, catalog_rel, &mut findings);
    check_row_values(&rows, policy, catalog_rel, &mut findings);
    check_file_parity(project_dir, &rows, policy, catalog_rel, &mut findings);
    let records = collect_records(project_dir, policy, &mut findings);
    check_record_agreement(&rows, &records, policy, catalog_rel, &mut findings);
    findings
}

fn drift(rel: &str, detail: &str) -> ScenarioFinding {
    ScenarioFinding {
        rule_id: RULE_CATALOG_RUNS_DRIFT,
        path: Some(rel.to_string()),
        message: format!("Scenario catalog: {rel} — {detail}"),
    }
}

/// One parsed catalog table row: the id from the File link target and
/// the Status / Gate cells.
struct CatalogRow {
    id: String,
    status: String,
    gate: Option<String>,
}

/// Parse every table data row in the catalog whose File cell carries a
/// markdown link.
fn parse_rows(
    content: &str, catalog_rel: &str, findings: &mut Vec<ScenarioFinding>,
) -> Vec<CatalogRow> {
    let link_re = Regex::new(r"^\[`?([a-z][a-z0-9-]*)`?\]\(([^)]+)\)$").expect("valid regex");
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
        let Some(caps) = link_re.captures(file_cell) else {
            findings.push(drift(
                catalog_rel,
                &format!("table row File cell '{file_cell}' does not parse as [`<id>`](<id>.md)"),
            ));
            continue;
        };
        let label = caps[1].to_string();
        let target = caps[2].to_string();
        let id = target.strip_suffix(".md").unwrap_or(&target).to_string();
        if label != id {
            findings.push(drift(
                catalog_rel,
                &format!("row label '{label}' disagrees with linked file '{target}'"),
            ));
        }
        rows.push(CatalogRow {
            id,
            status: cells.get(2).copied().unwrap_or_default().to_string(),
            gate: cells.get(3).map(|cell| (*cell).to_string()),
        });
    }
    rows
}

/// Per-row value checks: duplicate ids, Status against the legal set,
/// Gate against the legal set (when the policy declares one).
fn check_row_values(
    rows: &[CatalogRow], policy: &CatalogPolicy, catalog_rel: &str,
    findings: &mut Vec<ScenarioFinding>,
) {
    let mut seen: BTreeMap<&str, u32> = BTreeMap::new();
    for row in rows {
        *seen.entry(row.id.as_str()).or_default() += 1;
        if !policy.statuses.contains(&row.status) {
            findings.push(drift(
                catalog_rel,
                &format!(
                    "row '{id}' status '{status}' is not one of [{legal}]",
                    id = row.id,
                    status = row.status,
                    legal = policy.statuses.join(", ")
                ),
            ));
        }
        if policy.gates.is_empty() {
            continue;
        }
        match &row.gate {
            None => findings.push(drift(
                catalog_rel,
                &format!("row '{id}' is missing the Gate column", id = row.id),
            )),
            Some(gate) if !policy.gates.contains(gate) => findings.push(drift(
                catalog_rel,
                &format!(
                    "row '{id}' gate '{gate}' is not one of [{legal}]",
                    id = row.id,
                    legal = policy.gates.join(", ")
                ),
            )),
            Some(_) => {}
        }
    }
    for (id, count) in seen {
        if count > 1 {
            findings.push(drift(catalog_rel, &format!("duplicate catalog rows for '{id}'")));
        }
    }
}

/// Row↔scenario-file parity in both directions.
fn check_file_parity(
    project_dir: &Path, rows: &[CatalogRow], policy: &CatalogPolicy, catalog_rel: &str,
    findings: &mut Vec<ScenarioFinding>,
) {
    let file_ids = markdown_stems(&project_dir.join(&policy.scenarios_dir));
    let row_ids: BTreeSet<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    for row in rows {
        if !file_ids.contains(&row.id) {
            findings.push(drift(
                catalog_rel,
                &format!(
                    "row '{id}' has no scenario file {dir}/{id}.md",
                    id = row.id,
                    dir = policy.scenarios_dir
                ),
            ));
        }
    }
    for file_id in &file_ids {
        if !row_ids.contains(file_id.as_str()) {
            findings.push(ScenarioFinding {
                rule_id: RULE_CATALOG_RUNS_DRIFT,
                path: Some(format!("{dir}/{file_id}.md", dir = policy.scenarios_dir)),
                message: format!(
                    "Scenario catalog: {catalog_rel} — scenario file \
                     {dir}/{file_id}.md has no catalog row",
                    dir = policy.scenarios_dir
                ),
            });
        }
    }
}

/// Collect `<name>.md` stems (minus `README.md`) directly under `dir`.
fn markdown_stems(dir: &Path) -> BTreeSet<String> {
    let mut stems = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return stems;
    };
    for entry in entries.flatten() {
        if !entry.file_type().is_ok_and(|file_type| file_type.is_file()) {
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
            stems.insert(stem.to_string());
        }
    }
    stems
}

/// One committed run record parsed from its `<id>.<result>.md` filename.
struct RunRecord {
    id: String,
    result: String,
    rel: String,
}

/// Collect run records under the runs directory, flagging filenames
/// that do not parse as `<id>.<result>.md` or carry a `<result>` token
/// outside the policy's status↔result map.
fn collect_records(
    project_dir: &Path, policy: &CatalogPolicy, findings: &mut Vec<ScenarioFinding>,
) -> Vec<RunRecord> {
    let legal_results: BTreeSet<&str> =
        policy.status_result_map.values().map(String::as_str).collect();
    let mut records = Vec::new();
    for stem in markdown_stems(&project_dir.join(&policy.runs_dir)) {
        let rel = format!("{dir}/{stem}.md", dir = policy.runs_dir);
        let Some((id, result)) = stem.rsplit_once('.') else {
            findings.push(ScenarioFinding {
                rule_id: RULE_CATALOG_RUNS_DRIFT,
                path: Some(rel.clone()),
                message: format!(
                    "Scenario catalog: {rel} — run record filename must be <id>.<result>.md"
                ),
            });
            continue;
        };
        if !legal_results.contains(result) {
            findings.push(ScenarioFinding {
                rule_id: RULE_CATALOG_RUNS_DRIFT,
                path: Some(rel.clone()),
                message: format!(
                    "Scenario catalog: {rel} — record result '{result}' is not one of \
                     [{legal}]",
                    legal = policy
                        .status_result_map
                        .values()
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            });
            continue;
        }
        records.push(RunRecord {
            id: id.to_string(),
            result: result.to_string(),
            rel,
        });
    }
    records
}

/// Status↔record agreement: a status in the map requires exactly the
/// mapped record; a status outside the map (e.g. `pending`) must have
/// no record at all; at most one record per id; no orphan records.
fn check_record_agreement(
    rows: &[CatalogRow], records: &[RunRecord], policy: &CatalogPolicy, catalog_rel: &str,
    findings: &mut Vec<ScenarioFinding>,
) {
    let mut by_id: BTreeMap<&str, Vec<&RunRecord>> = BTreeMap::new();
    for record in records {
        by_id.entry(record.id.as_str()).or_default().push(record);
    }
    for (id, group) in &by_id {
        if group.len() > 1 {
            findings.push(drift(
                catalog_rel,
                &format!(
                    "multiple run records for '{id}': {names}",
                    names = group
                        .iter()
                        .map(|record| record.rel.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            ));
        }
    }
    let row_ids: BTreeSet<&str> = rows.iter().map(|row| row.id.as_str()).collect();
    for record in records {
        if !row_ids.contains(record.id.as_str()) {
            findings.push(ScenarioFinding {
                rule_id: RULE_CATALOG_RUNS_DRIFT,
                path: Some(record.rel.clone()),
                message: format!(
                    "Scenario catalog: {rel} — record names scenario '{id}' which has no \
                     catalog row",
                    rel = record.rel,
                    id = record.id
                ),
            });
        }
    }
    for row in rows {
        if !policy.statuses.contains(&row.status) {
            continue;
        }
        let group = by_id.get(row.id.as_str());
        match policy.status_result_map.get(&row.status) {
            Some(expected) => {
                let satisfied = group
                    .is_some_and(|records| records.iter().any(|record| record.result == *expected));
                if !satisfied {
                    findings.push(drift(
                        catalog_rel,
                        &format!(
                            "row '{id}' status '{status}' requires committed record \
                             {dir}/{id}.{expected}.md",
                            id = row.id,
                            status = row.status,
                            dir = policy.runs_dir
                        ),
                    ));
                }
            }
            None => {
                for record in group.into_iter().flatten() {
                    findings.push(drift(
                        catalog_rel,
                        &format!(
                            "record {rel} disagrees with the '{status}' row for '{id}'",
                            rel = record.rel,
                            status = row.status,
                            id = row.id
                        ),
                    ));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn policy() -> CatalogPolicy {
        serde_json::from_value(policy_json()).expect("valid policy")
    }

    fn policy_json() -> JsonValue {
        json!({
            "catalog": "evals/scenarios/README.md",
            "scenarios-dir": "evals/scenarios",
            "runs-dir": "evals/runs",
            "statuses": ["pending", "passed", "failed", "deferred"],
            "gates": ["release-blocker", "full"],
            "status-result-map": {"passed": "pass", "failed": "fail", "deferred": "deferred"}
        })
    }

    /// Write a catalog with the given rows, one scenario file per id in
    /// `scenario_ids`, and the named run-record files.
    fn write_tree(dir: &Path, catalog_rows: &[&str], scenario_ids: &[&str], records: &[&str]) {
        let scenarios = dir.join("evals/scenarios");
        let runs = dir.join("evals/runs");
        std::fs::create_dir_all(&scenarios).expect("mkdir scenarios");
        std::fs::create_dir_all(&runs).expect("mkdir runs");
        let mut catalog =
            String::from("| Scenario | File | Status | Gate |\n| --- | --- | --- | --- |\n");
        for row in catalog_rows {
            catalog.push_str(row);
            catalog.push('\n');
        }
        std::fs::write(scenarios.join("README.md"), catalog).expect("write catalog");
        for id in scenario_ids {
            std::fs::write(scenarios.join(format!("{id}.md")), "body\n").expect("write scenario");
        }
        std::fs::write(runs.join("README.md"), "runs\n").expect("write runs readme");
        for record in records {
            std::fs::write(runs.join(record), "record\n").expect("write record");
        }
    }

    fn messages(findings: &[ScenarioFinding]) -> Vec<&str> {
        findings.iter().map(|finding| finding.message.as_str()).collect()
    }

    #[test]
    fn clean_catalog_is_silent() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tree(
            dir.path(),
            &[
                "| One | [`alpha`](alpha.md) | passed | release-blocker |",
                "| Two | [`beta`](beta.md) | pending | full |",
            ],
            &["alpha", "beta"],
            &["alpha.pass.md"],
        );
        let findings = check_catalog_runs(dir.path(), &policy());
        assert!(findings.is_empty(), "unexpected findings: {:?}", messages(&findings));
    }

    #[test]
    fn flags_row_without_scenario_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tree(dir.path(), &["| One | [`ghost`](ghost.md) | pending | full |"], &[], &[]);
        let findings = check_catalog_runs(dir.path(), &policy());
        assert_eq!(messages(&findings).len(), 1);
        assert!(findings[0].message.contains("no scenario file evals/scenarios/ghost.md"));
    }

    #[test]
    fn flags_scenario_file_without_row() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tree(dir.path(), &[], &["orphan"], &[]);
        let findings = check_catalog_runs(dir.path(), &policy());
        assert_eq!(messages(&findings).len(), 1);
        assert!(findings[0].message.contains("evals/scenarios/orphan.md has no catalog row"));
        assert_eq!(findings[0].path.as_deref(), Some("evals/scenarios/orphan.md"));
    }

    #[test]
    fn flags_illegal_status_and_gate() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tree(
            dir.path(),
            &["| One | [`alpha`](alpha.md) | shipped | blocking |"],
            &["alpha"],
            &[],
        );
        let findings = check_catalog_runs(dir.path(), &policy());
        let texts = messages(&findings);
        assert!(texts.iter().any(|m| m.contains("status 'shipped' is not one of")));
        assert!(texts.iter().any(|m| m.contains("gate 'blocking' is not one of")));
    }

    #[test]
    fn flags_missing_gate_column() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tree(dir.path(), &["| One | [`alpha`](alpha.md) | pending |"], &["alpha"], &[]);
        let findings = check_catalog_runs(dir.path(), &policy());
        assert!(findings.iter().any(|f| f.message.contains("missing the Gate column")));
    }

    #[test]
    fn empty_gates_skips_gate_validation() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tree(dir.path(), &["| One | [`alpha`](alpha.md) | pending |"], &["alpha"], &[]);
        let mut relaxed = policy();
        relaxed.gates.clear();
        assert!(check_catalog_runs(dir.path(), &relaxed).is_empty());
    }

    #[test]
    fn status_bearing_row_requires_record() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tree(dir.path(), &["| One | [`alpha`](alpha.md) | passed | full |"], &["alpha"], &[]);
        let findings = check_catalog_runs(dir.path(), &policy());
        assert_eq!(messages(&findings).len(), 1);
        assert!(
            findings[0]
                .message
                .contains("status 'passed' requires committed record evals/runs/alpha.pass.md")
        );
    }

    #[test]
    fn record_result_must_agree_with_status() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tree(
            dir.path(),
            &["| One | [`alpha`](alpha.md) | failed | full |"],
            &["alpha"],
            &["alpha.pass.md"],
        );
        let findings = check_catalog_runs(dir.path(), &policy());
        assert!(findings.iter().any(|f| {
            f.message.contains("status 'failed' requires committed record evals/runs/alpha.fail.md")
        }));
    }

    #[test]
    fn pending_row_with_record_is_drift() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tree(
            dir.path(),
            &["| One | [`alpha`](alpha.md) | pending | full |"],
            &["alpha"],
            &["alpha.pass.md"],
        );
        let findings = check_catalog_runs(dir.path(), &policy());
        assert_eq!(messages(&findings).len(), 1);
        assert!(
            findings[0]
                .message
                .contains("record evals/runs/alpha.pass.md disagrees with the 'pending' row")
        );
    }

    #[test]
    fn flags_multiple_records_per_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tree(
            dir.path(),
            &["| One | [`alpha`](alpha.md) | passed | full |"],
            &["alpha"],
            &["alpha.pass.md", "alpha.fail.md"],
        );
        let findings = check_catalog_runs(dir.path(), &policy());
        assert!(findings.iter().any(|f| f.message.contains("multiple run records for 'alpha'")));
    }

    #[test]
    fn flags_orphan_and_unknown_result_records() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tree(
            dir.path(),
            &["| One | [`alpha`](alpha.md) | pending | full |"],
            &["alpha"],
            &["ghost.pass.md", "alpha.aced.md"],
        );
        let findings = check_catalog_runs(dir.path(), &policy());
        let texts = messages(&findings);
        assert!(texts.iter().any(|m| m.contains("record names scenario 'ghost'")));
        assert!(texts.iter().any(|m| m.contains("record result 'aced' is not one of")));
    }

    #[test]
    fn flags_duplicate_rows_and_label_drift() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_tree(
            dir.path(),
            &[
                "| One | [`alpha`](alpha.md) | pending | full |",
                "| Bis | [`alfa`](alpha.md) | pending | full |",
            ],
            &["alpha"],
            &[],
        );
        let findings = check_catalog_runs(dir.path(), &policy());
        let texts = messages(&findings);
        assert!(texts.iter().any(|m| m.contains("row label 'alfa' disagrees with linked file")));
        assert!(texts.iter().any(|m| m.contains("duplicate catalog rows for 'alpha'")));
    }

    #[test]
    fn missing_catalog_is_one_finding() {
        let dir = tempfile::tempdir().expect("tempdir");
        let findings = check_catalog_runs(dir.path(), &policy());
        assert_eq!(messages(&findings).len(), 1);
        assert!(findings[0].message.contains("cannot be read"));
    }

    #[test]
    fn config_parse_modes() {
        assert!(CatalogPolicy::parse(None).expect("absent ok").is_none());
        let unrelated = json!({"known-schemas": []});
        assert!(CatalogPolicy::parse(Some(&unrelated)).expect("unrelated ok").is_none());
        assert!(CatalogPolicy::parse(Some(&policy_json())).expect("full ok").is_some());
        let broken = json!({"catalog": "x.md"});
        CatalogPolicy::parse(Some(&broken)).expect_err("partial config must fail to parse");
    }

    #[test]
    fn invalid_config_is_a_finding() {
        let dir = tempfile::tempdir().expect("tempdir");
        let broken = json!({"catalog": "x.md"});
        let findings = findings_from_config(dir.path(), Some(&broken));
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_CATALOG_RUNS_DRIFT);
        assert!(findings[0].message.contains("invalid catalog-runs config"));
    }
}

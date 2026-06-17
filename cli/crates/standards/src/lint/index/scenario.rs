//! Dedicated scenario discovery + extraction pass per the standards
//! layer's scoped scenario fact family.
//!
//! Scenario files live partly under the un-indexed `evals/` tree,
//! so this pass walks the opt-in scenario roots itself rather than
//! reading [`super::framework`]'s file set, and emits a dedicated
//! [`Scenario`] fact family that is appended to the model WITHOUT
//! touching [`crate::lint::WorkspaceModel::files`]. Keeping scenario
//! files out of the file fact family means no other rule's
//! `path-pattern` candidate set changes (zero blast radius).
//!
//! [`discover_scenario_candidates`] is the **single** scenario
//! discovery — the in-process `scenarios` framework checker consumes it
//! too: flat `evals/scenarios/<id>.md` files (skipping the pack
//! `README.md`), `adapters/targets/<adapter>/tests/<file>.md` and
//! `.../tests/<dir>/scenario.md`, and
//! `plugins/<plugin>/skills/<skill>/fixtures/<case>/scenario.md`.
//! Symlinks are never traversed or collected (the host walks with
//! `follow_links(false)`). A file opts in only when it begins with a
//! `---` frontmatter block; an opted-in file whose YAML fails to parse
//! still emits a fact with empty [`Scenario::fields`] so the
//! `kind: schema` hint flags it.

use std::path::{Path, PathBuf};

use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::lint::Scenario;

/// Discover and parse every opt-in scenario under `project_dir`,
/// returning the [`Scenario`] facts sorted by path.
#[must_use]
pub fn extract(project_dir: &Path) -> Vec<Scenario> {
    let mut scenarios: Vec<Scenario> = discover_scenario_candidates(project_dir)
        .into_iter()
        .filter_map(|path| parse_scenario(project_dir, &path))
        .collect();
    scenarios.sort_by(|a, b| a.path.cmp(&b.path));
    scenarios
}

/// Parse one candidate into a [`Scenario`] fact, or `None` when the
/// file is unreadable or carries no leading frontmatter block (not
/// opted in).
fn parse_scenario(project_dir: &Path, path: &Path) -> Option<Scenario> {
    let content = std::fs::read_to_string(path).ok()?;
    let (block, body) = super::frontmatter::split(&content)?;
    let fields = parse_fields(block);
    let id = fields.get("id").and_then(JsonValue::as_str).map(str::to_owned);
    Some(Scenario {
        path: relative_display(project_dir, path),
        id,
        stages: string_array(&fields, "stages"),
        expected_artifacts: string_array(&fields, "expected-artifacts"),
        body_id: body_scenario_id(body),
        fields,
    })
}

/// Parse the frontmatter block into a field map. A YAML body that
/// fails to parse, or parses to a non-object, yields an empty map so a
/// `kind: schema` hint still flags the opted-in file.
fn parse_fields(block: &str) -> JsonMap<String, JsonValue> {
    match serde_saphyr::from_str::<JsonValue>(block) {
        Ok(JsonValue::Object(map)) => map,
        Ok(_) | Err(_) => JsonMap::new(),
    }
}

/// Collect the string entries of a frontmatter array field, dropping
/// non-string entries; empty when the field is absent or not an array.
fn string_array(fields: &JsonMap<String, JsonValue>, key: &str) -> Vec<String> {
    fields
        .get(key)
        .and_then(JsonValue::as_array)
        .map(|items| items.iter().filter_map(|v| v.as_str().map(str::to_owned)).collect())
        .unwrap_or_default()
}

/// Read the body `Scenario ID:` line value (backticks stripped), or
/// `None` when no such line is present.
fn body_scenario_id(body: &str) -> Option<String> {
    for line in body.lines() {
        if let Some(rest) = line.trim().strip_prefix("Scenario ID:") {
            let token = rest.trim().trim_matches('`').trim();
            if !token.is_empty() {
                return Some(token.to_owned());
            }
        }
    }
    None
}

/// Discover scenario candidate files under `root`.
///
/// Covers the eval scenario pack (`evals/scenarios/<id>.md`), target
/// adapter tests (`adapters/targets/<adapter>/tests/…`), and plugin
/// skill fixtures (`plugins/<plugin>/skills/<skill>/fixtures/<case>/scenario.md`).
/// Symlinks are never traversed or collected. The single scenario
/// discovery shared by this fact extractor and the in-process
/// `scenarios` framework checker.
#[must_use]
pub fn discover_scenario_candidates(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_evals(&root.join("evals").join("scenarios"), &mut out);
    collect_targets(&root.join("adapters").join("targets"), &mut out);
    collect_plugin_fixtures(&root.join("plugins"), &mut out);
    out.sort();
    out.dedup();
    out
}

/// Flat `evals/scenarios/<id>.md` files (depth 1), skipping the
/// pack `README.md` catalog.
fn collect_evals(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if !file_type.is_file() {
            continue;
        }
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name == "README.md" || !is_markdown(&path) {
            continue;
        }
        out.push(path);
    }
}

/// `adapters/targets/<adapter>/tests/<file>.md` and
/// `adapters/targets/<adapter>/tests/<dir>/scenario.md`.
fn collect_targets(targets_dir: &Path, out: &mut Vec<PathBuf>) {
    let mut files = Vec::new();
    walk_files(targets_dir, &mut files);
    for path in files {
        let Ok(rel) = path.strip_prefix(targets_dir) else {
            continue;
        };
        let parts: Vec<&str> = rel.iter().filter_map(|c| c.to_str()).collect();
        if is_markdown(&path) && parts.len() == 3 && parts[1] == "tests" {
            out.push(path.clone());
        }
        if parts.len() == 4 && parts[1] == "tests" && parts[3] == "scenario.md" {
            out.push(path);
        }
    }
}

/// `plugins/<plugin>/skills/<skill>/fixtures/<case>/scenario.md`.
fn collect_plugin_fixtures(plugins_dir: &Path, out: &mut Vec<PathBuf>) {
    let mut files = Vec::new();
    walk_files(plugins_dir, &mut files);
    for path in files {
        if path.file_name().and_then(|n| n.to_str()) != Some("scenario.md") {
            continue;
        }
        let Ok(rel) = path.strip_prefix(plugins_dir) else {
            continue;
        };
        let parts: Vec<&str> = rel.iter().filter_map(|c| c.to_str()).collect();
        if parts.len() == 6
            && parts[1] == "skills"
            && parts[3] == "fixtures"
            && parts[5] == "scenario.md"
        {
            out.push(path);
        }
    }
}

/// Recursive file collector that never follows or records symlinks,
/// matching the host's `follow_links(false)` + symlink-skip posture.
fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            walk_files(&path, out);
        } else if file_type.is_file() {
            out.push(path);
        }
    }
}

fn is_markdown(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

fn relative_display(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

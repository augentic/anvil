//! Skill-manifest predicates over `plugins/<plugin>/skills/<skill>/SKILL.md`:
//! frontmatter presence, schema validity, name uniqueness, the plugin
//! discovery-prefix invariant, and the ultrathin-wrapper bans
//! (frontmatter restatement, orchestration headings).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value as JsonValue;

use crate::support::{Finding, discover_skills, parse_frontmatter};

/// A SKILL.md has no parseable, non-empty frontmatter block.
pub const CHECK_MISSING_FRONTMATTER: &str = "skill.missing-frontmatter";
/// A SKILL.md frontmatter block fails `skill.schema.json`.
pub const CHECK_SCHEMA_VIOLATION: &str = "skill.schema-violation";
/// Two SKILL.md files declare the same `name:`.
pub const CHECK_DUPLICATE_NAME: &str = "skill.duplicate-name";
/// A skill `name` does not carry its plugin's discovery prefix.
pub const CHECK_NAME_PREFIX: &str = "skill.name-directory-mismatch";
/// A skill body restates frontmatter with a `## Input` H2.
pub const CHECK_FRONTMATTER_RESTATEMENT: &str = "skill.frontmatter-restatement";
/// A spec skill body carries an orchestration/judgment heading.
pub const CHECK_ORCHESTRATION_HEADING: &str = "skill.orchestration-heading";

/// Plugin directories whose published discovery prefix differs from
/// the directory name.
const PREFIX_OVERRIDES: &[(&str, &str)] = &[("spec", "specify")];

/// Heading terms naming engine-owned behavior. A spec skill is an
/// invoke-and-relay wrapper: orchestration, synthesis, validation, and
/// lifecycle mechanics live in the `specify` CLI and its embedded
/// judgment prose, so a body heading about them signals prose that
/// belongs to the workflow crate or an adapter, not the wrapper.
const ORCHESTRATION_HEADING_TERMS: &[&str] = &[
    "synthes",
    "reconcil",
    "orchestrat",
    "validat",
    "lifecycle",
    "extraction",
    "judgment",
    "algorithm",
];

static SKILL_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]*$").expect("skill name pattern"));
static INPUT_HEADING_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^## Input\s*$").expect("input heading pattern"));

/// Run every skill predicate rooted at `root`.
pub fn run(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut names: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (plugin, _skill, relative) in discover_skills(root) {
        let Ok(content) = fs::read_to_string(root.join(&relative)) else {
            continue;
        };

        let frontmatter = parse_frontmatter(&content);
        let fields = match frontmatter {
            Some(fields) if !fields.is_empty() => fields,
            _ => {
                findings.push(Finding::new(
                    CHECK_MISSING_FRONTMATTER,
                    format!("{relative} — missing, unparseable, or empty frontmatter"),
                ));
                continue;
            }
        };

        check_schema(&relative, &fields, &mut findings);
        check_input_heading(&relative, &content, &mut findings);
        if plugin == "spec" {
            check_orchestration_headings(&relative, &content, &mut findings);
        }

        if let Some(name) = fields.get("name").and_then(JsonValue::as_str) {
            let name = name.trim();
            if !name.is_empty() {
                names.entry(name.to_owned()).or_default().push(relative.clone());
                check_name_prefix(&relative, name, &plugin, &mut findings);
            }
        }
    }

    for (name, paths) in names {
        if paths.len() > 1 {
            findings.push(Finding::new(
                CHECK_DUPLICATE_NAME,
                format!("skill name '{name}' declared by more than one SKILL.md: {paths:?}"),
            ));
        }
    }

    findings
}

/// Validate frontmatter against the embedded `skill.schema.json`.
fn check_schema(
    relative: &str, fields: &serde_json::Map<String, JsonValue>, findings: &mut Vec<Finding>,
) {
    let validator = match schema::cached_validator(schema::SKILL_JSON_SCHEMA) {
        Ok(validator) => validator,
        Err(error) => {
            findings.push(Finding::new(
                CHECK_SCHEMA_VIOLATION,
                format!("{relative} — cannot compile skill.schema.json: {error}"),
            ));
            return;
        }
    };
    let value = JsonValue::Object(fields.clone());
    for error in validator.iter_errors(&value) {
        findings.push(Finding::new(
            CHECK_SCHEMA_VIOLATION,
            format!("{relative} — frontmatter violates skill.schema.json at {}: {error}", {
                error.instance_path()
            }),
        ));
    }
}

/// Every well-formed skill `name` must begin with its plugin's
/// discovery prefix (`<plugin>-`), modulo the override map.
fn check_name_prefix(relative: &str, name: &str, plugin: &str, findings: &mut Vec<Finding>) {
    if !SKILL_NAME_RE.is_match(name) {
        return;
    }
    let base = PREFIX_OVERRIDES
        .iter()
        .find(|(dir, _)| *dir == plugin)
        .map_or(plugin, |(_, prefix)| prefix);
    let required = format!("{base}-");
    if !name.starts_with(&required) {
        findings.push(Finding::new(
            CHECK_NAME_PREFIX,
            format!("{relative} — skill name '{name}' must start with '{required}'"),
        ));
    }
}

/// A spec skill body heading must not name engine-owned behavior
/// (fenced code blocks are exempt — they quote CLI output, not body
/// structure).
fn check_orchestration_headings(relative: &str, content: &str, findings: &mut Vec<Finding>) {
    let mut in_fence = false;
    for (line_idx, line) in content.lines().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || !line.starts_with('#') {
            continue;
        }
        let lower = line.to_ascii_lowercase();
        if let Some(term) = ORCHESTRATION_HEADING_TERMS.iter().find(|term| lower.contains(**term)) {
            findings.push(Finding::new(
                CHECK_ORCHESTRATION_HEADING,
                format!(
                    "{relative}:{} — heading '{}' names engine-owned behavior ('{term}'); spec \
                     skills are invoke-and-relay wrappers",
                    line_idx + 1,
                    line.trim()
                ),
            ));
        }
    }
}

/// A skill body must not restate the rendered `argument-hint`
/// frontmatter with a dedicated `## Input` H2.
fn check_input_heading(relative: &str, content: &str, findings: &mut Vec<Finding>) {
    for (line_idx, line) in content.lines().enumerate() {
        if INPUT_HEADING_RE.is_match(line) {
            findings.push(Finding::new(
                CHECK_FRONTMATTER_RESTATEMENT,
                format!(
                    "{relative}:{} — '## Input' restates frontmatter rendered on every invocation",
                    line_idx + 1
                ),
            ));
        }
    }
}

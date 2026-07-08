//! Skill-manifest predicates over `plugins/<plugin>/skills/<skill>/SKILL.md`:
//! frontmatter presence, schema validity, name uniqueness, the plugin
//! discovery-prefix invariant, the argument-hint and description
//! grammars, and the frontmatter-restatement ban.

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
/// An `argument-hint` token violates the slash-command grammar.
pub const CHECK_ARGUMENT_HINT: &str = "skill.argument-hint-grammar";
/// A `description` does not start with an approved imperative verb.
pub const CHECK_DESCRIPTION_VERB: &str = "skill.description-grammar";
/// A skill body restates frontmatter with a `## Input` H2.
pub const CHECK_FRONTMATTER_RESTATEMENT: &str = "skill.frontmatter-restatement";

/// Plugin directories whose published discovery prefix differs from
/// the directory name.
const PREFIX_OVERRIDES: &[(&str, &str)] = &[("spec", "specify")];

/// Closed slash-command argument grammar: `<name>`, `[name]`, `<a|b>`,
/// `[a|b]`, optional `...`, or `--flag`, with kebab-case names.
const ARGUMENT_HINT_TOKEN_PATTERN: &str = r"^(?:<[a-z][a-z0-9]*(?:-[a-z0-9]+)*(?:\|[a-z][a-z0-9]*(?:-[a-z0-9]+)*)*>(?:\.\.\.)?|\[[a-z][a-z0-9]*(?:-[a-z0-9]+)*(?:\|[a-z][a-z0-9]*(?:-[a-z0-9]+)*)*\](?:\.\.\.)?|--[a-z][a-z0-9]*(?:-[a-z0-9]+)*)$";

/// Approved imperative verbs a skill `description` may start with.
const DESCRIPTION_VERBS: &[&str] = &[
    "add",
    "annotate",
    "apply",
    "audit",
    "author",
    "build",
    "categorise",
    "categorize",
    "check",
    "compare",
    "compile",
    "complete",
    "compose",
    "compute",
    "configure",
    "convert",
    "create",
    "decompose",
    "define",
    "describe",
    "design",
    "diff",
    "discover",
    "drive",
    "drop",
    "enforce",
    "execute",
    "expose",
    "export",
    "extract",
    "fetch",
    "fix",
    "format",
    "generate",
    "guard",
    "implement",
    "import",
    "infer",
    "ingest",
    "init",
    "initialize",
    "list",
    "load",
    "merge",
    "monitor",
    "orchestrate",
    "plan",
    "preview",
    "process",
    "produce",
    "propose",
    "publish",
    "reconstruct",
    "refine",
    "render",
    "resolve",
    "review",
    "run",
    "scaffold",
    "select",
    "show",
    "shorten",
    "split",
    "stage",
    "store",
    "summarize",
    "test",
    "translate",
    "transform",
    "trim",
    "validate",
    "verify",
    "wire",
    "wrap",
    "write",
];

static SKILL_NAME_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z][a-z0-9-]*$").expect("skill name pattern"));
static ARGUMENT_HINT_TOKEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(ARGUMENT_HINT_TOKEN_PATTERN).expect("argument-hint grammar"));
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
        check_argument_hint(&relative, &fields, &mut findings);
        check_description_verb(&relative, &fields, &mut findings);
        check_input_heading(&relative, &content, &mut findings);

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
    let validator = match specify_schema::cached_validator(specify_schema::SKILL_JSON_SCHEMA) {
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

/// Every whitespace-separated `argument-hint` token must match the
/// closed grammar; a present non-string value is flagged outright.
fn check_argument_hint(
    relative: &str, fields: &serde_json::Map<String, JsonValue>, findings: &mut Vec<Finding>,
) {
    let Some(value) = fields.get("argument-hint") else {
        return;
    };
    let Some(text) = value.as_str() else {
        findings.push(Finding::new(
            CHECK_ARGUMENT_HINT,
            format!("{relative} — frontmatter field 'argument-hint' must be a string"),
        ));
        return;
    };
    if let Some(token) =
        text.split_whitespace().find(|token| !ARGUMENT_HINT_TOKEN_RE.is_match(token))
    {
        findings.push(Finding::new(
            CHECK_ARGUMENT_HINT,
            format!(
                "{relative} — argument-hint token '{token}' (in '{text}') does not match the \
                 slash-command grammar"
            ),
        ));
    }
}

/// The first alphabetic word of `description` (lowercased) must be a
/// member of the approved verb allow-list.
fn check_description_verb(
    relative: &str, fields: &serde_json::Map<String, JsonValue>, findings: &mut Vec<Finding>,
) {
    let Some(text) = fields.get("description").and_then(JsonValue::as_str) else {
        return;
    };
    let first_word = text.split_whitespace().next().unwrap_or("");
    let first_alpha: String = first_word.chars().take_while(char::is_ascii_alphabetic).collect();
    if first_alpha.is_empty() {
        findings.push(Finding::new(
            CHECK_DESCRIPTION_VERB,
            format!("{relative} — description has no leading alphabetic word"),
        ));
        return;
    }
    let lower = first_alpha.to_ascii_lowercase();
    if !DESCRIPTION_VERBS.contains(&lower.as_str()) {
        findings.push(Finding::new(
            CHECK_DESCRIPTION_VERB,
            format!(
                "{relative} — description first word '{first_alpha}' is not an approved \
                 imperative verb"
            ),
        ));
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

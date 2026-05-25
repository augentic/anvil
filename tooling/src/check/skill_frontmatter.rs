use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde_json::Value as JsonValue;

use crate::context::Context;
use crate::error::ToolingError;
use crate::finding::{Check, Finding, Location};
use crate::helpers::{relative_display, skill_frontmatter, walk_skill_files};
use crate::schema::{validate_frontmatter, SchemaError, SchemaId};

pub const RULE_SCHEMA_VIOLATION: &str = "skill.schema-violation";
pub const RULE_MISSING_FRONTMATTER: &str = "skill.missing-frontmatter";
pub const RULE_NAME_DIRECTORY_MISMATCH: &str = "skill.name-directory-mismatch";
pub const RULE_DUPLICATE_NAME: &str = "skill.duplicate-name";
pub const RULE_UNKNOWN_TOOL: &str = "skill.unknown-tool";
pub const RULE_DESCRIPTION_GRAMMAR: &str = "skill.description-grammar";
pub const RULE_ARGUMENT_HINT_GRAMMAR: &str = "skill.argument-hint-grammar";

/// Kept in sync with `tooling/schemas/skill.schema.json` (`description.maxLength`).
pub const MAX_DESCRIPTION_CHARS: usize = 512;

const PREFIX_OVERRIDES: &[(&str, &str)] = &[("spec", "specify")];

/// Validate SKILL.md frontmatter against `tooling/schemas/skill.schema.json`.
pub struct SkillFrontmatterSchemaCheck;

/// Require `name:` to carry the containing plugin's discovery prefix.
pub struct SkillNameDirectoryMismatchCheck;

/// Require globally unique skill `name:` values.
pub struct SkillDuplicateNameCheck;

/// Whitelist `allowed-tools` entries against the known Cursor tool set.
pub struct SkillUnknownToolCheck;

/// Require `description:` to start with a curated imperative verb.
pub struct SkillDescriptionGrammarCheck;

/// Enforce the canonical `argument-hint:` token grammar with rich diagnostics.
pub struct SkillArgumentHintGrammarCheck;

impl Check for SkillFrontmatterSchemaCheck {
    fn run(&self, ctx: &Context) -> Vec<Finding> {
        check_schema(ctx).unwrap_or_else(|error| {
            vec![infrastructure_finding(RULE_SCHEMA_VIOLATION, error)]
        })
    }
}

impl Check for SkillNameDirectoryMismatchCheck {
    fn run(&self, ctx: &Context) -> Vec<Finding> {
        check_name_directory_mismatch(ctx).unwrap_or_else(|error| {
            vec![infrastructure_finding(RULE_NAME_DIRECTORY_MISMATCH, error)]
        })
    }
}

impl Check for SkillDuplicateNameCheck {
    fn run(&self, ctx: &Context) -> Vec<Finding> {
        check_duplicate_names(ctx).unwrap_or_else(|error| {
            vec![infrastructure_finding(RULE_DUPLICATE_NAME, error)]
        })
    }
}

impl Check for SkillUnknownToolCheck {
    fn run(&self, ctx: &Context) -> Vec<Finding> {
        check_unknown_tools(ctx).unwrap_or_else(|error| {
            vec![infrastructure_finding(RULE_UNKNOWN_TOOL, error)]
        })
    }
}

impl Check for SkillDescriptionGrammarCheck {
    fn run(&self, ctx: &Context) -> Vec<Finding> {
        check_description_grammar(ctx).unwrap_or_else(|error| {
            vec![infrastructure_finding(RULE_DESCRIPTION_GRAMMAR, error)]
        })
    }
}

impl Check for SkillArgumentHintGrammarCheck {
    fn run(&self, ctx: &Context) -> Vec<Finding> {
        check_argument_hint_grammar(ctx).unwrap_or_else(|error| {
            vec![infrastructure_finding(RULE_ARGUMENT_HINT_GRAMMAR, error)]
        })
    }
}

struct SkillEntry {
    path: PathBuf,
    rel: String,
    plugin_dir: String,
    frontmatter: Option<BTreeMap<String, JsonValue>>,
}

fn load_skill_entries(ctx: &Context) -> Result<Vec<SkillEntry>, ToolingError> {
    let framework_root = ctx.framework_root();
    let plugins_dir = ctx.plugins_dir();

    walk_skill_files(framework_root)?
        .into_iter()
        .map(|path| {
            let rel = relative_display(framework_root, &path);
            let plugin_dir = path
                .strip_prefix(&plugins_dir)
                .ok()
                .and_then(|rel| rel.components().next())
                .map(|component| component.as_os_str().to_string_lossy().into_owned())
                .unwrap_or_default();
            let content = fs::read_to_string(&path)?;
            let frontmatter = skill_frontmatter(&content);
            Ok(SkillEntry {
                path,
                rel,
                plugin_dir,
                frontmatter,
            })
        })
        .collect()
}

fn check_schema(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let mut findings = Vec::new();

    for entry in load_skill_entries(ctx)? {
        match validate_frontmatter(ctx, &entry.path, SchemaId::Skill) {
            Ok(()) => {}
            Err(SchemaError::Infrastructure(error)) => {
                return Err(error);
            }
            Err(SchemaError::Validation(errors)) => {
                for error in errors {
                    let rule_id = if error.message.contains("missing leading YAML frontmatter") {
                        RULE_MISSING_FRONTMATTER
                    } else {
                        RULE_SCHEMA_VIOLATION
                    };
                    findings.push(finding(
                        rule_id,
                        format!(
                            "Skill frontmatter: {} — {} {}",
                            entry.rel, error.instance_path, error.message
                        ),
                        &entry.path,
                    ));
                }
            }
        }
    }

    Ok(findings)
}

fn check_name_directory_mismatch(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let name_re = Regex::new(r"^[a-z][a-z0-9-]*$").expect("name regex");
    let mut findings = Vec::new();

    for entry in load_skill_entries(ctx)? {
        let Some(frontmatter) = &entry.frontmatter else {
            continue;
        };

        let Some(name) = frontmatter.get("name").and_then(JsonValue::as_str) else {
            continue;
        };

        if !name_re.is_match(name) {
            continue;
        }

        let prefix_base = prefix_override(&entry.plugin_dir).unwrap_or(entry.plugin_dir.as_str());
        let required_prefix = format!("{prefix_base}-");
        if name.starts_with(&required_prefix) {
            continue;
        }

        findings.push(finding(
            RULE_NAME_DIRECTORY_MISMATCH,
            format!(
                "Skill name missing plugin prefix: {} — '{name}' must start with '{required_prefix}'",
                entry.rel
            ),
            &entry.path,
        ));
    }

    Ok(findings)
}

fn check_duplicate_names(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let mut names_by_value: HashMap<String, Vec<String>> = HashMap::new();

    for entry in load_skill_entries(ctx)? {
        let Some(frontmatter) = &entry.frontmatter else {
            continue;
        };
        let Some(name) = frontmatter.get("name").and_then(JsonValue::as_str) else {
            continue;
        };
        names_by_value
            .entry(name.to_string())
            .or_default()
            .push(entry.rel.clone());
    }

    let mut findings = Vec::new();
    for (name, paths) in names_by_value {
        if paths.len() <= 1 {
            continue;
        }
        let path = ctx
            .framework_root()
            .join(paths[0].replace('/', std::path::MAIN_SEPARATOR_STR));
        findings.push(finding(
            RULE_DUPLICATE_NAME,
            format!(
                "Duplicate skill name '{name}' across SKILL.md files: {}",
                paths.join(", ")
            ),
            &path,
        ));
    }

    Ok(findings)
}

fn check_unknown_tools(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let mut findings = Vec::new();

    for entry in load_skill_entries(ctx)? {
        let Some(frontmatter) = &entry.frontmatter else {
            continue;
        };
        let Some(tools) = frontmatter.get("allowed-tools").and_then(JsonValue::as_str) else {
            continue;
        };

        for tool in tools.split_whitespace().map(str::trim).filter(|t| !t.is_empty()) {
            if known_tool(tool) {
                continue;
            }
            findings.push(finding(
                RULE_UNKNOWN_TOOL,
                format!("Unknown tool in allowed-tools: {} — '{tool}'", entry.rel),
                &entry.path,
            ));
        }
    }

    Ok(findings)
}

fn check_description_grammar(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let mut findings = Vec::new();

    for entry in load_skill_entries(ctx)? {
        let Some(frontmatter) = &entry.frontmatter else {
            continue;
        };
        let Some(description) = frontmatter.get("description").and_then(JsonValue::as_str) else {
            continue;
        };

        let trimmed = description.trim_start();
        let Some(first_word) = trimmed.split_whitespace().next() else {
            findings.push(finding(
                RULE_DESCRIPTION_GRAMMAR,
                format!(
                    "Skill description must start with an imperative verb: {} — no leading word found",
                    entry.rel
                ),
                &entry.path,
            ));
            continue;
        };

        let first_alpha = first_word
            .chars()
            .take_while(|ch| ch.is_ascii_alphabetic())
            .collect::<String>();
        if first_alpha.is_empty() {
            findings.push(finding(
                RULE_DESCRIPTION_GRAMMAR,
                format!(
                    "Skill description must start with an imperative verb: {} — no leading word found",
                    entry.rel
                ),
                &entry.path,
            ));
            continue;
        }

        let lower = first_alpha.to_ascii_lowercase();
        if imperative_verbs().contains(&lower.as_str()) {
            continue;
        }

        findings.push(finding(
            RULE_DESCRIPTION_GRAMMAR,
            format!(
                "Skill description must start with an imperative verb: {} — '{first_alpha}' not in allow-list (add to IMPERATIVE_VERBS in tooling/src/check/skill_frontmatter.rs if it is genuinely imperative)",
                entry.rel
            ),
            &entry.path,
        ));
    }

    Ok(findings)
}

fn check_argument_hint_grammar(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let mut findings = Vec::new();

    for entry in load_skill_entries(ctx)? {
        let Some(frontmatter) = &entry.frontmatter else {
            continue;
        };
        let Some(hint_value) = frontmatter.get("argument-hint") else {
            continue;
        };
        let Some(hint) = hint_value.as_str() else {
            findings.push(finding(
                RULE_ARGUMENT_HINT_GRAMMAR,
                format!(
                    "Invalid argument-hint type in {}: must be a string",
                    entry.rel
                ),
                &entry.path,
            ));
            continue;
        };

        if let Some(message) = argument_hint_grammar_error(hint) {
            findings.push(finding(
                RULE_ARGUMENT_HINT_GRAMMAR,
                format!(
                    "Invalid argument-hint in {}: token '{message}' (in '{hint}') does not match grammar — allowed tokens are <name>, [name], <a|b>, [a|b], <name>..., [name]..., --flag (kebab-case names)",
                    entry.rel
                ),
                &entry.path,
            ));
        }
    }

    Ok(findings)
}

/// Pure per-hint predicate for tests and richer diagnostics than JSON Schema alone.
pub fn argument_hint_grammar_error(hint: &str) -> Option<String> {
    let trimmed = hint.trim();
    if trimmed.is_empty() {
        return None;
    }

    let token_re = argument_hint_token_regex();
    for token in trimmed.split_whitespace() {
        if !token_re.is_match(token) {
            return Some(token.to_string());
        }
    }
    None
}

fn argument_hint_token_regex() -> Regex {
    let hint_name = r"[a-z][a-z0-9]*(?:-[a-z0-9]+)*";
    let hint_alt = format!("{hint_name}(?:\\|{hint_name})*");
    Regex::new(&format!(
        "^(?:<{hint_alt}>(?:\\.\\.\\.)?|\\[{hint_alt}\\](?:\\.\\.\\.)?|--[a-z][a-z0-9]*(?:-[a-z0-9]+)*)$"
    ))
    .expect("argument-hint token regex")
}

fn prefix_override(plugin_dir: &str) -> Option<&'static str> {
    PREFIX_OVERRIDES
        .iter()
        .find(|(dir, _)| *dir == plugin_dir)
        .map(|(_, prefix)| *prefix)
}

fn known_tool(tool: &str) -> bool {
    tool.starts_with("mcp__") || KNOWN_TOOLS.contains(&tool)
}

const KNOWN_TOOLS: &[&str] = &[
    "Read",
    "Write",
    "StrReplace",
    "Shell",
    "Grep",
    "Glob",
    "ReadLints",
    "WebFetch",
    "WebSearch",
    "AskQuestion",
    "Task",
    "TodoWrite",
    "SemanticSearch",
    "EditNotebook",
    "GenerateImage",
];

fn imperative_verbs() -> &'static [&'static str] {
    &[
        "add", "annotate", "apply", "audit", "author", "build", "categorise", "categorize", "check",
        "compare", "compile", "complete", "compose", "compute", "configure", "convert", "create",
        "decompose", "define", "describe", "design", "diff", "discover", "drive", "drop", "enforce",
        "execute", "expose", "export", "extract", "fetch", "fix", "format", "generate", "guard",
        "implement", "import", "infer", "ingest", "init", "initialize", "list", "load", "merge",
        "monitor", "orchestrate", "plan", "preview", "process", "produce", "propose", "publish",
        "reconstruct", "refine", "render", "resolve", "review", "run", "scaffold", "select", "show",
        "shorten", "split", "stage", "store", "summarize", "test", "translate", "transform", "trim",
        "validate", "verify", "wire", "wrap", "write",
    ]
}

fn finding(rule_id: &'static str, message: impl Into<String>, path: &Path) -> Finding {
    Finding {
        rule_id,
        message: message.into(),
        location: Some(Location {
            path: path.to_path_buf(),
            line: 1,
            column: None,
        }),
    }
}

fn infrastructure_finding(rule_id: &'static str, error: ToolingError) -> Finding {
    Finding {
        rule_id,
        message: error.to_string(),
        location: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argument_hint_grammar_accepts_valid_tokens() {
        assert!(argument_hint_grammar_error("<slice-dir>").is_none());
        assert!(argument_hint_grammar_error("[crate-name]").is_none());
        assert!(argument_hint_grammar_error("<a|b|c>").is_none());
        assert!(argument_hint_grammar_error("--kind <kind>").is_none());
    }

    #[test]
    fn argument_hint_grammar_rejects_bare_prose() {
        assert_eq!(
            argument_hint_grammar_error("the slice name"),
            Some("the".to_string())
        );
    }
}

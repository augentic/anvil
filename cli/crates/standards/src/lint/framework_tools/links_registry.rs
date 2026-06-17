//! In-process `links-registry` framework checker (Road B `kind: tool`).
//!
//! Covers CORE-018 (`schemas.specify.dev` URLs in adapter briefs must
//! resolve to a known tool-owned schema; the tool→schema registry is
//! CORE-018's forwarded `config:`) and CORE-020 (`<!-- skill:
//! plugin:skill -->` directives must resolve against the on-disk skill
//! registry under `plugins/`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use serde_json::Value as JsonValue;

use super::support::{ToolFinding, parsed_config, relative_display, requested_rule, walk_files};

const RULE_BRIEF_SCHEMA_LINK_RESOLVE: &str = "CORE-018";
const RULE_UNRESOLVED_DIRECTIVE: &str = "CORE-020";

const RULES: &[&str] = &[RULE_BRIEF_SCHEMA_LINK_RESOLVE, RULE_UNRESOLVED_DIRECTIVE];

/// One tool→schema-name registry row the CORE-018 rule supplies in
/// `config:`.
#[derive(Debug, Clone, PartialEq, Eq)]
struct KnownSchema {
    tool: String,
    schemas: Vec<String>,
}

/// One link-registry violation before wire guidance is attached.
#[derive(Debug, Clone, PartialEq, Eq)]
struct LinkFinding {
    rule_id: &'static str,
    path: Option<String>,
    message: String,
}

/// Run the link-registry family scoped by the candidate sentinel path.
pub fn run(project_dir: &Path, args: &[String]) -> Vec<ToolFinding> {
    let scoped = requested_rule(args, RULES);
    let config = parsed_config(args);

    let mut findings = Vec::new();
    if scoped.is_none() || scoped == Some(RULE_BRIEF_SCHEMA_LINK_RESOLVE) {
        let registry = known_schemas(config.as_ref());
        findings.extend(check_schema_links(project_dir, &registry));
    }
    if scoped.is_none() || scoped == Some(RULE_UNRESOLVED_DIRECTIVE) {
        findings.extend(check_directives(project_dir));
    }
    findings.into_iter().map(wire_finding).collect()
}

fn wire_finding(finding: LinkFinding) -> ToolFinding {
    let (impact, remediation) = guidance(finding.rule_id);
    ToolFinding {
        rule_id: finding.rule_id,
        path: finding.path,
        message: finding.message,
        impact,
        remediation,
    }
}

fn guidance(rule_id: &str) -> (&'static str, &'static str) {
    match rule_id {
        RULE_BRIEF_SCHEMA_LINK_RESOLVE => (
            "An adapter brief references a schemas.specify.dev URL that does not resolve to a known tool-owned schema, so readers follow a dead link.",
            "Point the URL at a schema named in the rule's known-schemas registry, or register the schema with its owning tool first.",
        ),
        _ => (
            "A skill directive references a plugin or skill that does not exist on disk, so the directive cannot resolve at runtime.",
            "Fix the `<!-- skill: plugin:skill -->` directive to name an existing plugin and skill under plugins/.",
        ),
    }
}

/// Parse CORE-018's tool→schema registry out of the forwarded
/// `config.known-schemas` array. Rows missing fields are dropped.
fn known_schemas(config: Option<&JsonValue>) -> Vec<KnownSchema> {
    config
        .and_then(|value| value.get("known-schemas"))
        .and_then(JsonValue::as_array)
        .map(|rows| rows.iter().filter_map(known_schema_row).collect())
        .unwrap_or_default()
}

fn known_schema_row(row: &JsonValue) -> Option<KnownSchema> {
    let tool = row.get("tool").and_then(JsonValue::as_str)?;
    let schemas = row
        .get("schemas")
        .and_then(JsonValue::as_array)?
        .iter()
        .filter_map(|s| s.as_str().map(str::to_string))
        .collect();
    Some(KnownSchema {
        tool: tool.to_string(),
        schemas,
    })
}

/// CORE-018: every `https://schemas.specify.dev/<tool>/<name>.schema.json`
/// URL in an adapter brief or reference must resolve to a known
/// tool-owned schema named in the supplied `registry`. URLs in fenced or
/// inline code are ignored.
fn check_schema_links(project_dir: &Path, registry: &[KnownSchema]) -> Vec<LinkFinding> {
    let adapters_dir = project_dir.join("adapters");
    if !adapters_dir.is_dir() {
        return Vec::new();
    }

    let url_re = schema_url_pattern();
    let inline_re = inline_code_pattern();
    let mut findings = Vec::new();

    for path in walk_markdown(&adapters_dir) {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let rel = relative_display(project_dir, &path);

        let mut in_fence = false;
        for (line_idx, line) in content.lines().enumerate() {
            if line.trim_start().starts_with("```") {
                in_fence = !in_fence;
                continue;
            }
            if in_fence {
                continue;
            }
            let cleaned = inline_re.replace_all(line, "");
            for cap in url_re.captures_iter(&cleaned) {
                let tool = cap.get(1).map_or("", |m| m.as_str());
                let name_with_ext = cap.get(2).map_or("", |m| m.as_str());
                let name = name_with_ext.strip_suffix(".schema.json").unwrap_or(name_with_ext);
                if !is_known_schema(registry, tool, name) {
                    let url = cap.get(0).map_or("", |m| m.as_str());
                    findings.push(LinkFinding {
                        rule_id: RULE_BRIEF_SCHEMA_LINK_RESOLVE,
                        path: Some(rel.clone()),
                        message: format!(
                            "{rel}:{} — schema URL '{url}' does not resolve to a known \
                             tool-owned schema",
                            line_idx + 1,
                        ),
                    });
                }
            }
        }
    }

    findings
}

fn is_known_schema(registry: &[KnownSchema], tool: &str, name: &str) -> bool {
    registry.iter().any(|row| row.tool == tool && row.schemas.iter().any(|s| s == name))
}

/// CORE-020: every `<!-- skill: plugin:skill -->` directive across the
/// tree must resolve against the skill registry discovered under
/// `plugins/`. Directives in fenced or inline code are ignored.
fn check_directives(project_dir: &Path) -> Vec<LinkFinding> {
    let directive_re = directive_pattern();
    let fence_re = fenced_code_pattern();
    let inline_re = inline_code_pattern();
    let registry = build_skill_registry(project_dir);

    let mut findings = Vec::new();
    for path in walk_markdown(project_dir) {
        let path_str = path.to_string_lossy().replace('\\', "/");
        if skip_path(&path_str) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let rel = relative_display(project_dir, &path);

        let no_fence = fence_re.replace_all(&content, "");
        let stripped = inline_re.replace_all(&no_fence, "").into_owned();

        for cap in directive_re.captures_iter(&stripped) {
            let plugin = cap.get(1).map_or("", |m| m.as_str());
            let skill = cap.get(2).map_or("", |m| m.as_str());
            match registry.get(plugin) {
                None => findings.push(LinkFinding {
                    rule_id: RULE_UNRESOLVED_DIRECTIVE,
                    path: Some(rel.clone()),
                    message: format!(
                        "Invalid skill directive: {rel} — plugin '{plugin}' not found"
                    ),
                }),
                Some(skills) if !skills.contains(skill) => findings.push(LinkFinding {
                    rule_id: RULE_UNRESOLVED_DIRECTIVE,
                    path: Some(rel.clone()),
                    message: format!(
                        "Invalid skill directive: {rel} — skill '{plugin}:{skill}' not found"
                    ),
                }),
                _ => {}
            }
        }
    }

    findings
}

/// Discover the `plugin -> {skill}` registry from
/// `plugins/<plugin>/skills/<skill>/SKILL.md` paths.
fn build_skill_registry(project_dir: &Path) -> BTreeMap<String, BTreeSet<String>> {
    let plugins_dir = project_dir.join("plugins");
    let mut files = Vec::new();
    walk_files(&plugins_dir, &mut files);
    let mut registry: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for path in files {
        if path.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
            continue;
        }
        let Ok(rel) = path.strip_prefix(&plugins_dir) else {
            continue;
        };
        let parts: Vec<&str> = rel.iter().filter_map(|c| c.to_str()).collect();
        if parts.len() >= 4 && parts[1] == "skills" {
            registry.entry(parts[0].to_string()).or_default().insert(parts[2].to_string());
        }
    }
    registry
}

/// Mirror the directive predicate's skip set: vendored / VCS / scratch
/// trees and the host's own test fixtures are never scanned.
fn skip_path(path: &str) -> bool {
    path.contains("node_modules")
        || path.contains("/.git/")
        || path.contains("temp")
        || path.contains("tooling/tests/fixtures")
        || path.contains("crates/standards/tests/fixtures")
}

/// Collect every `.md` file under `dir`, skipping symlinked paths.
fn walk_markdown(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_files(dir, &mut files);
    let mut out: Vec<PathBuf> = files
        .into_iter()
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect();
    out.sort();
    out
}

fn schema_url_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"https://schemas\.specify\.dev/([a-z][a-z0-9-]*)/([a-z][a-z0-9-]*\.schema\.json)",
        )
        .expect("valid schema URL pattern")
    })
}

fn directive_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"<!-- skill: ([a-z][a-z0-9-]*):([a-z][a-z0-9-]*) -->")
            .expect("valid directive pattern")
    })
}

fn fenced_code_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"```[\s\S]*?```").expect("valid fence pattern"))
}

fn inline_code_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"`[^`]+`").expect("valid inline code pattern"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vectis_registry() -> Vec<KnownSchema> {
        vec![KnownSchema {
            tool: "vectis".to_string(),
            schemas: vec!["tokens".to_string(), "assets".to_string(), "composition".to_string()],
        }]
    }

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, body).expect("write");
    }

    #[test]
    fn schema_links_flag_unknown_tool_schema() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "adapters/targets/vectis/briefs/build.md",
            "See https://schemas.specify.dev/vectis/unknown.schema.json for details.\n",
        );
        let findings = check_schema_links(dir.path(), &vectis_registry());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_BRIEF_SCHEMA_LINK_RESOLVE);
    }

    #[test]
    fn schema_links_accept_known_schema() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(
            dir.path(),
            "adapters/targets/vectis/briefs/build.md",
            "See https://schemas.specify.dev/vectis/tokens.schema.json for details.\n",
        );
        assert!(check_schema_links(dir.path(), &vectis_registry()).is_empty());
    }

    #[test]
    fn directives_flag_unknown_plugin() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "docs/guide.md", "<!-- skill: ghost:refine -->\n");
        let findings = check_directives(dir.path());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_UNRESOLVED_DIRECTIVE);
        assert!(findings[0].message.contains("plugin 'ghost' not found"));
    }

    #[test]
    fn directives_accept_registered_skill() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "plugins/spec/skills/refine/SKILL.md", "---\nname: refine\n---\n");
        write(dir.path(), "docs/guide.md", "<!-- skill: spec:refine -->\n");
        assert!(check_directives(dir.path()).is_empty());
    }

    #[test]
    fn directives_flag_unknown_skill_in_known_plugin() {
        let dir = tempfile::tempdir().expect("tempdir");
        write(dir.path(), "plugins/spec/skills/refine/SKILL.md", "---\nname: refine\n---\n");
        write(dir.path(), "docs/guide.md", "<!-- skill: spec:ghost -->\n");
        let findings = check_directives(dir.path());
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("skill 'spec:ghost' not found"));
    }
}

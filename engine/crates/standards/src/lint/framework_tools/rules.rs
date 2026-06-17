//! In-process `rules` framework checker (Road B `kind: tool`).
//!
//! Covers the whole-tree rule-tree family: CORE-009 namespace
//! ownership, CORE-026 duplicate rule id, CORE-053 body heading. All
//! policy (the owner→prefix map, source-axis prefixes, and
//! reserved-namespace owners) arrives in CORE-009's forwarded
//! `config:`; only the filesystem mechanism (the axis layout, the
//! shared `universal` / `core` packs, the `<adapter>/rules/` shape) is
//! code.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use serde_json::Value as JsonValue;

use super::support::{ToolFinding, parsed_config, relative_display, requested_rule, walk_files};

const RULE_NAMESPACE_OWNERSHIP_VIOLATION: &str = "CORE-009";
const RULE_DUPLICATE_RULE_ID: &str = "CORE-026";
const RULE_BODY_HEADING_MISSING: &str = "CORE-053";

const RULES: &[&str] =
    &[RULE_NAMESPACE_OWNERSHIP_VIOLATION, RULE_DUPLICATE_RULE_ID, RULE_BODY_HEADING_MISSING];

/// Shared `universal` / `core` rules-pack directory names under
/// `adapters/shared/rules/`. Mechanism (filesystem layout), not policy.
const SHARED_PACKS: [&str; 2] = ["universal", "core"];

/// `specify`-owned namespace policy CORE-009 supplies in `config:`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct OwnerPolicy {
    owner_prefixes: BTreeMap<String, BTreeSet<String>>,
    source_axis_prefixes: BTreeSet<String>,
    reserved: BTreeMap<String, String>,
}

/// Run the rule-tree family scoped by the candidate sentinel path.
pub fn run(project_dir: &Path, args: &[String]) -> Vec<ToolFinding> {
    let scoped = requested_rule(args, RULES);
    let config = parsed_config(args);

    let mut findings = Vec::new();
    if scoped.is_none() || scoped == Some(RULE_NAMESPACE_OWNERSHIP_VIOLATION) {
        // No owner policy supplied means nothing to compare against; emit
        // a clean report rather than treating every owner as unknown.
        if let Some(policy) = parse_policy(config.as_ref()) {
            findings.extend(check_namespace_ownership(project_dir, &policy));
        }
    }
    if scoped.is_none() || scoped == Some(RULE_DUPLICATE_RULE_ID) {
        findings.extend(check_duplicate_rule_id(project_dir));
    }
    if scoped.is_none() || scoped == Some(RULE_BODY_HEADING_MISSING) {
        findings.extend(check_rule_body_heading(project_dir));
    }
    findings
}

/// Build the CORE-009 namespace policy from the forwarded `config:`;
/// `None` when the required `owner-prefixes` map is absent.
fn parse_policy(config: Option<&JsonValue>) -> Option<OwnerPolicy> {
    let config = config?;
    let owner_prefixes = parse_prefix_map(config.get("owner-prefixes")?)?;
    let source_axis_prefixes =
        config.get("source-axis-prefixes").map(parse_string_set).unwrap_or_default();
    let reserved = config.get("reserved-namespaces").map(parse_string_map).unwrap_or_default();
    Some(OwnerPolicy {
        owner_prefixes,
        source_axis_prefixes,
        reserved,
    })
}

fn parse_prefix_map(value: &JsonValue) -> Option<BTreeMap<String, BTreeSet<String>>> {
    let object = value.as_object()?;
    let mut map = BTreeMap::new();
    for (owner, prefixes) in object {
        map.insert(owner.clone(), parse_string_set(prefixes));
    }
    Some(map)
}

fn parse_string_map(value: &JsonValue) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    if let Some(object) = value.as_object() {
        for (key, raw) in object {
            if let Some(text) = raw.as_str() {
                map.insert(key.clone(), text.to_string());
            }
        }
    }
    map
}

fn parse_string_set(value: &JsonValue) -> BTreeSet<String> {
    let mut set = BTreeSet::new();
    if let Some(array) = value.as_array() {
        for raw in array {
            if let Some(text) = raw.as_str() {
                set.insert(text.to_string());
            }
        }
    }
    set
}

fn ownership_finding(rel: &str, message: String) -> ToolFinding {
    ToolFinding {
        rule_id: RULE_NAMESPACE_OWNERSHIP_VIOLATION,
        path: Some(rel.to_string()),
        message,
        impact: "A rule's id-namespace prefix is not owned by the rules directory it lives under, so the codex namespace ownership invariant is broken.",
        remediation: "Move the rule into the directory that owns its namespace prefix, or renumber the id to the prefix its current directory owns.",
    }
}

/// CORE-009: assert each rule file's id-namespace prefix is owned by its
/// containing rules directory.
fn check_namespace_ownership(project_dir: &Path, policy: &OwnerPolicy) -> Vec<ToolFinding> {
    let mut findings = Vec::new();
    for path in discover_rule_files(project_dir) {
        let rel = relative_display(project_dir, &path);
        let Some(frontmatter) = read_frontmatter(&path) else {
            continue;
        };
        let Some(id) = frontmatter.get("id").and_then(JsonValue::as_str) else {
            continue;
        };
        let Some(owner) = owner_for_path(project_dir, &path) else {
            continue;
        };
        let Some(namespace) = namespace_prefix(id) else {
            continue;
        };

        if let Some(reserved_owner) = policy.reserved.get(namespace)
            && owner != *reserved_owner
        {
            findings.push(ownership_finding(&rel, format!(
                "Rules namespace ownership: {rel} — {namespace}-* ids are reserved for framework-repo declarative rules and may not be placed under adapter trees (got '{id}' under rules owner '{owner}')"
            )));
            continue;
        }

        let Some(allowed) = allowed_prefixes(project_dir, &path, &owner, policy) else {
            findings.push(ownership_finding(&rel, format!(
                "Rules namespace ownership: {rel} — rules owner '{owner}' has no configured namespace; add it to the CORE-009 rule's owner-prefixes config before adding first-party rules here"
            )));
            continue;
        };

        if !allowed.contains(namespace) {
            findings.push(ownership_finding(
                &rel,
                format!(
                    "Rules namespace ownership: {rel} — rules owner '{owner}' may only use {} ids, got '{id}'",
                    namespace_list(allowed)
                ),
            ));
        }
    }
    findings
}

/// CORE-026: a rule id declared in more than one rules markdown file is a
/// whole-tree duplicate.
fn check_duplicate_rule_id(project_dir: &Path) -> Vec<ToolFinding> {
    let mut ids_by_value: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for path in discover_rule_files(project_dir) {
        let rel = relative_display(project_dir, &path);
        let Some(frontmatter) = read_frontmatter(&path) else {
            continue;
        };
        let Some(id) = frontmatter.get("id").and_then(JsonValue::as_str) else {
            continue;
        };
        ids_by_value.entry(id.to_string()).or_default().push(rel);
    }

    let mut findings = Vec::new();
    for (id, mut paths) in ids_by_value {
        if paths.len() > 1 {
            paths.sort();
            findings.push(ToolFinding {
                rule_id: RULE_DUPLICATE_RULE_ID,
                path: None,
                message: format!("Rule duplicate id '{id}' across files: {}", paths.join(", ")),
                impact: "The same rule id appears in more than one rules markdown file, so codex consumers cannot resolve a single rule.",
                remediation: "Rename the colliding rules so each frontmatter id is unique across the rules tree.",
            });
        }
    }
    findings
}

/// CORE-053: every rules markdown file's body must carry the verbatim
/// `## Rule` heading on its own line.
fn check_rule_body_heading(project_dir: &Path) -> Vec<ToolFinding> {
    let mut findings = Vec::new();
    for path in discover_rule_files(project_dir) {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        if !has_rule_heading(&content) {
            let rel = relative_display(project_dir, &path);
            findings.push(ToolFinding {
                rule_id: RULE_BODY_HEADING_MISSING,
                path: Some(rel.clone()),
                message: format!(
                    "Rule body heading: {rel} — rule markdown must carry a verbatim `## Rule` heading in its body"
                ),
                impact: "A rule markdown file's body is missing the `## Rule` heading, so reviewing agents cannot locate the policy text.",
                remediation: "Add a verbatim `## Rule` heading on its own line above the rule's policy statement.",
            });
        }
    }
    findings
}

fn has_rule_heading(content: &str) -> bool {
    body_after_frontmatter(content).lines().any(|line| line == "## Rule")
}

fn body_after_frontmatter(content: &str) -> &str {
    let Some(rest) = content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n"))
    else {
        return content;
    };
    let Some(end) = rest.find("\n---") else {
        return content;
    };
    let after_fence = &rest[end + 1..];
    after_fence.find('\n').map_or("", |newline| &after_fence[newline + 1..])
}

/// Resolve the allowed id-prefix set for a rule file: the source-axis
/// prefixes for a source-adapter rule (dynamic owner discovery), else
/// the static `owner_prefixes` entry for the directory owner.
fn allowed_prefixes<'a>(
    project_dir: &Path, path: &Path, owner: &str, policy: &'a OwnerPolicy,
) -> Option<&'a BTreeSet<String>> {
    if is_source_rule(project_dir, path) {
        return Some(&policy.source_axis_prefixes);
    }
    policy.owner_prefixes.get(owner)
}

/// Discover every rules markdown file under the target / source axes
/// (`<adapter>/rules/**`) and the shared `universal` / `core` packs,
/// skipping `README.md`.
fn discover_rule_files(project_dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    for axis in ["sources", "targets"] {
        let axis_dir = project_dir.join("adapters").join(axis);
        let mut files = Vec::new();
        walk_files(&axis_dir, &mut files);
        for path in files {
            if !is_markdown(&path) || is_rules_readme(&path) {
                continue;
            }
            if is_rule_in_axis(&path, &axis_dir) {
                paths.push(path);
            }
        }
    }
    for pack in SHARED_PACKS {
        let pack_dir = project_dir.join("adapters").join("shared").join("rules").join(pack);
        let mut files = Vec::new();
        walk_files(&pack_dir, &mut files);
        for path in files {
            if !is_markdown(&path) || is_rules_readme(&path) {
                continue;
            }
            paths.push(path);
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn is_rule_in_axis(path: &Path, axis_root: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(axis_root) else {
        return false;
    };
    let parts = normal_parts(rel);
    parts.len() >= 3 && parts.get(1).copied() == Some("rules")
}

fn owner_for_path(project_dir: &Path, path: &Path) -> Option<String> {
    for axis in ["sources", "targets"] {
        let axis_dir = project_dir.join("adapters").join(axis);
        if let Ok(rel) = path.strip_prefix(&axis_dir) {
            let parts = normal_parts(rel);
            if parts.len() >= 3 && parts.get(1).copied() == Some("rules") {
                return parts.first().map(|part| (*part).to_string());
            }
        }
    }
    for pack in SHARED_PACKS {
        let pack_dir = project_dir.join("adapters").join("shared").join("rules").join(pack);
        if path.strip_prefix(&pack_dir).is_ok() {
            return Some(pack.to_string());
        }
    }
    None
}

fn is_source_rule(project_dir: &Path, path: &Path) -> bool {
    let axis_dir = project_dir.join("adapters").join("sources");
    is_rule_in_axis(path, &axis_dir)
}

/// Extract the `PREFIX` from a `PREFIX-NNN` rule id; `None` for any
/// other shape so malformed ids are left to the schema rule.
fn namespace_prefix(id: &str) -> Option<&str> {
    let (prefix, suffix) = id.split_once('-')?;
    let well_formed = !prefix.is_empty()
        && prefix.bytes().all(|b| b.is_ascii_uppercase())
        && suffix.len() == 3
        && suffix.bytes().all(|b| b.is_ascii_digit());
    well_formed.then_some(prefix)
}

fn namespace_list(namespaces: &BTreeSet<String>) -> String {
    namespaces.iter().map(|namespace| format!("{namespace}-*")).collect::<Vec<_>>().join(", ")
}

fn is_markdown(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| ext.eq_ignore_ascii_case("md"))
}

fn is_rules_readme(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("readme.md"))
}

fn normal_parts(rel: &Path) -> Vec<&str> {
    rel.components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .collect()
}

fn read_frontmatter(path: &Path) -> Option<BTreeMap<String, JsonValue>> {
    let content = std::fs::read_to_string(path).ok()?;
    let block = frontmatter_block(&content)?;
    serde_saphyr::from_str(block).ok()
}

fn frontmatter_block(content: &str) -> Option<&str> {
    let rest = content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n"))?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_rule(root: &Path, rel: &str, id: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("rule parent")).expect("mkdir");
        let body = format!(
            "---\nid: {id}\ntitle: Fixture\nseverity: optional\ntrigger: Namespace fixture.\n---\n\n## Rule\n\nBody.\n"
        );
        std::fs::write(path, body).expect("write rule");
    }

    fn write_rule_without_heading(root: &Path, rel: &str, id: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().expect("rule parent")).expect("mkdir");
        let body = format!(
            "---\nid: {id}\ntitle: Fixture\nseverity: optional\ntrigger: Namespace fixture.\n---\n\nBody without the heading.\n"
        );
        std::fs::write(path, body).expect("write rule");
    }

    fn policy() -> OwnerPolicy {
        OwnerPolicy {
            owner_prefixes: BTreeMap::from([
                ("universal".to_string(), BTreeSet::from(["UNI".to_string()])),
                ("core".to_string(), BTreeSet::from(["CORE".to_string()])),
                (
                    "omnia".to_string(),
                    BTreeSet::from(["OMNIA".to_string(), "RUST".to_string(), "SEC".to_string()]),
                ),
                ("vectis".to_string(), BTreeSet::from(["VECTIS".to_string()])),
            ]),
            source_axis_prefixes: BTreeSet::from(["SRC".to_string()]),
            reserved: BTreeMap::from([("FRAME".to_string(), "universal".to_string())]),
        }
    }

    #[test]
    fn clean_tree_is_silent() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_rule(dir.path(), "adapters/shared/rules/core/CORE-001.md", "CORE-001");
        write_rule(dir.path(), "adapters/shared/rules/universal/UNI-001.md", "UNI-001");
        write_rule(dir.path(), "adapters/targets/omnia/rules/OMNIA-001.md", "OMNIA-001");
        write_rule(dir.path(), "adapters/sources/documentation/rules/SRC-001.md", "SRC-001");
        assert!(check_namespace_ownership(dir.path(), &policy()).is_empty());
        assert!(check_duplicate_rule_id(dir.path()).is_empty());
        assert!(check_rule_body_heading(dir.path()).is_empty());
    }

    #[test]
    fn flags_missing_rule_heading() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_rule(dir.path(), "adapters/shared/rules/core/CORE-001.md", "CORE-001");
        write_rule_without_heading(
            dir.path(),
            "adapters/shared/rules/universal/UNI-001.md",
            "UNI-001",
        );
        let findings = check_rule_body_heading(dir.path());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_BODY_HEADING_MISSING);
        assert_eq!(findings[0].path.as_deref(), Some("adapters/shared/rules/universal/UNI-001.md"));
        assert!(findings[0].message.contains("`## Rule`"));
    }

    #[test]
    fn flags_misplaced_prefix() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_rule(dir.path(), "adapters/targets/omnia/rules/VECTIS-001.md", "VECTIS-001");
        let findings = check_namespace_ownership(dir.path(), &policy());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_NAMESPACE_OWNERSHIP_VIOLATION);
        assert!(findings[0].message.contains("rules owner 'omnia' may only use"));
        assert!(findings[0].message.contains("VECTIS-001"));
    }

    #[test]
    fn flags_reserved_frame_under_adapter() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_rule(dir.path(), "adapters/targets/omnia/rules/FRAME-001.md", "FRAME-001");
        let findings = check_namespace_ownership(dir.path(), &policy());
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("FRAME-* ids are reserved"));
        assert!(findings[0].message.contains("FRAME-001"));
    }

    #[test]
    fn flags_unknown_owner() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_rule(dir.path(), "adapters/targets/newadapter/rules/OMNIA-001.md", "OMNIA-001");
        let findings = check_namespace_ownership(dir.path(), &policy());
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("has no configured namespace"));
    }

    #[test]
    fn flags_duplicate_id() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_rule(dir.path(), "adapters/shared/rules/core/first.md", "CORE-100");
        write_rule(dir.path(), "adapters/shared/rules/core/second.md", "CORE-100");
        let findings = check_duplicate_rule_id(dir.path());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, RULE_DUPLICATE_RULE_ID);
        assert!(findings[0].message.contains("CORE-100"));
        assert!(findings[0].message.contains("first.md"));
        assert!(findings[0].message.contains("second.md"));
    }
}

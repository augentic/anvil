use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use regex::Regex;
use walkdir::WalkDir;

use crate::context::Context;
use crate::finding::{Check, Finding, Location};
use crate::helpers::{strip_html_comments, under_symlink, walk_skill_files};

const RULE_UNRESOLVED: &str = "links.unresolved";
const RULE_BROKEN_REFERENCE: &str = "links.broken-reference";
const RULE_UNRESOLVED_DIRECTIVE: &str = "links.unresolved-directive";

/// Markdown link resolution, skill reference checks, and skill directive validation.
pub struct LinksCheck;

impl Check for LinksCheck {
    fn id(&self) -> &'static str {
        "links"
    }

    fn run(&self, ctx: &Context) -> Vec<Finding> {
        run(ctx)
    }
}

/// Run all link predicates against `ctx`.
pub fn run(ctx: &Context) -> Vec<Finding> {
    run_on_root(ctx.framework_root())
}

/// Run all link predicates against a framework root (used by integration tests).
pub fn run_on_root(root: &Path) -> Vec<Finding> {
    let mut findings = check_markdown_links(root);
    findings.extend(check_references(root));
    findings.extend(check_directives(root));
    findings
}

fn check_markdown_links(root: &Path) -> Vec<Finding> {
    let skip = markdown_link_skip_patterns();
    let link_re = link_pattern();
    let fence_re = fenced_code_pattern();
    let inline_re = inline_code_pattern();

    let mut findings = Vec::new();
    for path in walk_markdown_files(root).unwrap_or_default() {
        let path_str = path_for_match(&path);
        if path_matches_any(&path_str, skip)
            || skip_rfc_markdown_path(&path_str)
            || skip_test_fixtures(&path_str)
        {
            continue;
        }

        let rel_file = path_relative(root, &path);
        let parent = path.parent().unwrap_or(root);
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        let stripped = {
            let no_fence = fence_re.replace_all(&content, "");
            let no_comments = strip_html_comments(&no_fence);
            inline_re.replace_all(&no_comments, "").into_owned()
        };

        for cap in link_re.captures_iter(&stripped) {
            let target = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            if target.starts_with("http://")
                || target.starts_with("https://")
                || target.starts_with("mailto:")
                || target.starts_with('#')
            {
                continue;
            }
            let path_part = target.split('#').next().unwrap_or("");
            if path_part.is_empty() || path_part.starts_with("src/") {
                continue;
            }
            let resolved = parent.join(path_part);
            if !resolved.exists() {
                findings.push(finding(
                    RULE_UNRESOLVED,
                    format!("Broken link in {rel_file}: {target}"),
                    path.clone(),
                ));
            }
        }
    }

    findings
}

fn check_references(root: &Path) -> Vec<Finding> {
    let ref_re = reference_link_pattern();
    let fence_re = fenced_code_pattern();
    let plugins_dir = root.join("plugins");

    let mut findings = Vec::new();
    let skill_files = walk_skill_files(root).unwrap_or_default();
    for path in skill_files {
        let rel = path_relative(root, &path);
        let skill_dir = path.parent().unwrap_or(&plugins_dir);
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(source) => {
                findings.push(finding(
                    RULE_BROKEN_REFERENCE,
                    format!("Skill reference missing: {rel} — cannot read SKILL.md: {source}"),
                    path.clone(),
                ));
                continue;
            }
        };

        let stripped = fence_re.replace_all(&content, "");
        for cap in ref_re.captures_iter(&stripped) {
            let ref_path = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let ref_path = ref_path.split('#').next().unwrap_or("");
            if ref_path.is_empty() {
                continue;
            }
            let resolved = skill_dir.join(ref_path);
            if !resolved.exists() {
                findings.push(finding(
                    RULE_BROKEN_REFERENCE,
                    format!(
                        "Skill reference missing: {rel} links to '{ref_path}' but it doesn't exist"
                    ),
                    path.clone(),
                ));
            }
        }
    }

    findings
}

fn check_directives(root: &Path) -> Vec<Finding> {
    let directive_re = directive_pattern();
    let fence_re = fenced_code_pattern();
    let inline_re = inline_code_pattern();
    let skip = directive_skip_patterns();
    let registry = build_skill_registry(root);

    let mut findings = Vec::new();
    for path in walk_markdown_files(root).unwrap_or_default() {
        let path_str = path_for_match(&path);
        if path_matches_any(&path_str, skip) || skip_test_fixtures(&path_str) {
            continue;
        }

        let rel = path_relative(root, &path);
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => continue,
        };

        let stripped = {
            let no_fence = fence_re.replace_all(&content, "");
            inline_re.replace_all(&no_fence, "").into_owned()
        };

        for cap in directive_re.captures_iter(&stripped) {
            let plugin = cap.get(1).map(|m| m.as_str()).unwrap_or("");
            let skill = cap.get(2).map(|m| m.as_str()).unwrap_or("");
            match registry.get(plugin) {
                None => findings.push(finding(
                    RULE_UNRESOLVED_DIRECTIVE,
                    format!("Invalid skill directive: {rel} — plugin '{plugin}' not found"),
                    path.clone(),
                )),
                Some(skills) if !skills.contains(skill) => findings.push(finding(
                    RULE_UNRESOLVED_DIRECTIVE,
                    format!("Invalid skill directive: {rel} — skill '{plugin}:{skill}' not found"),
                    path.clone(),
                )),
                _ => {}
            }
        }
    }

    findings
}

fn build_skill_registry(root: &Path) -> HashMap<String, HashSet<String>> {
    let plugins_dir = root.join("plugins");
    let mut registry: HashMap<String, HashSet<String>> = HashMap::new();

    for entry in WalkDir::new(&plugins_dir)
        .follow_links(false)
        .into_iter()
        .flatten()
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if path.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
            continue;
        }
        let rel = path
            .strip_prefix(&plugins_dir)
            .map(|p| p.to_string_lossy().replace('\\', "/"))
            .unwrap_or_default();
        let parts: Vec<&str> = rel.split('/').collect();
        if parts.len() >= 4 && parts[1] == "skills" {
            let plugin = parts[0].to_string();
            let skill = parts[2].to_string();
            registry.entry(plugin).or_default().insert(skill);
        }
    }

    registry
}

fn walk_markdown_files(root: &Path) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut out = Vec::new();
    for entry in WalkDir::new(root).follow_links(false).into_iter() {
        let entry = entry?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
            continue;
        }
        if under_symlink(root, &path).unwrap_or(true) {
            continue;
        }
        out.push(path);
    }
    out.sort();
    Ok(out)
}

fn markdown_link_skip_patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        [
            r"node_modules",
            r"\.git",
            r"temp",
            r"specify-cli",
            r"rfcs/done",
            r"rfcs/future",
            r"rfcs/next",
        ]
        .iter()
        .map(|pat| Regex::new(pat).expect("valid skip pattern"))
        .collect()
    })
}

/// Skip archived/speculative RFC paths while keeping in-force `rfcs/rfc-25*` prose.
fn skip_rfc_markdown_path(path: &str) -> bool {
    let Some(idx) = path.find("rfcs/rfc-") else {
        return false;
    };
    let after = &path[idx + "rfcs/rfc-".len()..];
    !after.starts_with("25")
}

fn skip_test_fixtures(path: &str) -> bool {
    path.contains("tooling/tests/fixtures")
}

fn directive_skip_patterns() -> &'static [Regex] {
    static PATTERNS: OnceLock<Vec<Regex>> = OnceLock::new();
    PATTERNS.get_or_init(|| {
        ["node_modules", r"\.git", "temp", "rfcs"]
            .iter()
            .map(|pat| Regex::new(pat).expect("valid skip pattern"))
            .collect()
    })
}

fn path_matches_any(path: &str, patterns: &[Regex]) -> bool {
    patterns.iter().any(|re| re.is_match(path))
}

fn path_for_match(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn path_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .unwrap_or_else(|_| path.display().to_string())
}

fn link_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[[^\]]*\]\(([^)]+)\)").expect("valid link pattern"))
}

fn reference_link_pattern() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"\[[^\]]*\]\((references/[^)]+|examples/[^)]+)\)")
            .expect("valid reference link pattern")
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

fn finding(rule_id: &'static str, message: String, path: PathBuf) -> Finding {
    Finding {
        rule_id,
        message,
        location: Some(Location {
            path,
            line: 1,
            column: None,
        }),
    }
}

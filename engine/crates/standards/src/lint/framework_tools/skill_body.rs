//! In-process `skill-body` framework checker (Road B `kind: tool`).
//!
//! Covers the filesystem-only skill-body family: CORE-040
//! (critical-path shape), CORE-046 (step-body duplicates critical
//! path), and CORE-048 (`$VAR` definition / reference coverage). The
//! line threshold, item bounds, and built-in variable allow-list are
//! `specify`-owned policy read from the rule's forwarded `config:`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use super::support::{
    ToolFinding, parsed_config, relative_display, requested_rule, string_array_field, usize_field,
    walk_files,
};

const RULE_INVALID_CRITICAL_PATH: &str = "CORE-040";
const RULE_STEP_BODY_DUPLICATES: &str = "CORE-046";
const RULE_VARIABLE_COVERAGE: &str = "CORE-048";

const RULES: &[&str] =
    &[RULE_INVALID_CRITICAL_PATH, RULE_STEP_BODY_DUPLICATES, RULE_VARIABLE_COVERAGE];

/// The H2 section a long skill must carry / whose entries must not be
/// duplicated by step bodies. Mechanism, not a tunable policy value.
const CRITICAL_PATH_HEADING: &str = "## Critical Path";

static LIST_ITEM_RE: LazyLock<Regex> = LazyLock::new(|| cached(r"^(?:\d+\.|-)\s+\S"));
static STEP_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| cached(r"^Step\s+\d+\s*[:.\-]\s*"));
static LIST_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| cached(r"^(?:\d+\.|-|\*)\s+"));
static HEADING_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| cached(r"^#{2,4}\s+"));
static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| cached(r"\s+"));
static LIST_LINE_RE: LazyLock<Regex> = LazyLock::new(|| cached(r"^(?:\d+\.|-|\*)\s+\S"));
static VAR_DEF_RE: LazyLock<Regex> = LazyLock::new(|| cached(r"(?m)^\$([A-Z_][A-Z_0-9]*)\s*="));
static VAR_USE_RE: LazyLock<Regex> = LazyLock::new(|| cached(r"\$([A-Z_][A-Z_0-9]*)"));
static ARGS_HEADING_RE: LazyLock<Regex> =
    LazyLock::new(|| cached(r"(?m)^## (?:Derived )?Arguments"));
static TEXT_CODE_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| cached(r"```text\n([\s\S]*?)```"));
static FENCE_STRIP_RE: LazyLock<Regex> = LazyLock::new(|| cached(r"```[\s\S]*?```"));
static INLINE_CODE_RE: LazyLock<Regex> = LazyLock::new(|| cached(r"`[^`]+`"));
static ALL_CAPS_VAR_RE: LazyLock<Regex> = LazyLock::new(|| cached(r"^[A-Z][A-Z_]+$"));

fn cached(pattern: &str) -> Regex {
    Regex::new(pattern)
        .unwrap_or_else(|err| unreachable!("static skill-body regex must compile: {err}"))
}

/// One skill-body violation before wire guidance is attached.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SkillBodyFinding {
    rule_id: &'static str,
    path: String,
    message: String,
}

/// Run the skill-body family scoped by the candidate sentinel path.
pub fn run(project_dir: &Path, args: &[String]) -> Vec<ToolFinding> {
    let scoped = requested_rule(args, RULES);
    let config = parsed_config(args);
    let findings = match scoped {
        Some(RULE_INVALID_CRITICAL_PATH) => check_invalid_critical_path(
            project_dir,
            usize_field(config.as_ref(), "min-body-lines"),
            usize_field(config.as_ref(), "min-items"),
            usize_field(config.as_ref(), "max-items"),
        ),
        Some(RULE_STEP_BODY_DUPLICATES) => check_step_body_duplicates(project_dir),
        Some(RULE_VARIABLE_COVERAGE) => check_variable_coverage(
            project_dir,
            &string_array_field(config.as_ref(), "builtin-vars"),
        ),
        _ => Vec::new(),
    };
    findings.into_iter().map(wire_finding).collect()
}

fn wire_finding(finding: SkillBodyFinding) -> ToolFinding {
    let (impact, remediation) = guidance(finding.rule_id);
    ToolFinding {
        rule_id: finding.rule_id,
        path: Some(finding.path),
        message: finding.message,
        impact,
        remediation,
    }
}

fn guidance(rule_id: &str) -> (&'static str, &'static str) {
    match rule_id {
        RULE_INVALID_CRITICAL_PATH => (
            "A long skill's `## Critical Path` section does not list the required number of steps, so the table of contents is not a faithful map of the skill body.",
            "Rewrite the `## Critical Path` section to list the configured number of bullets or numbered steps.",
        ),
        RULE_STEP_BODY_DUPLICATES => (
            "A step body repeats a `## Critical Path` entry verbatim, duplicating the table of contents instead of pointing to references.",
            "Keep step bodies as short pointers to references; do not restate the Critical Path entries.",
        ),
        _ => (
            "A skill defines a `$VAR` in its Arguments section that is never referenced (or references one that is never defined), so the variable contract is inconsistent.",
            "Reference every defined `$VAR` in the body and define every `$VAR` the body uses in the Arguments section.",
        ),
    }
}

/// CORE-040: a skill whose body is at least `min_body_lines` long and
/// carries a `## Critical Path` section must list between `min_items`
/// and `max_items` entries (list or H3 form).
fn check_invalid_critical_path(
    project_dir: &Path, min_body_lines: usize, min_items: usize, max_items: usize,
) -> Vec<SkillBodyFinding> {
    let mut findings = Vec::new();
    for path in walk_skill_files(project_dir) {
        let rel = relative_display(project_dir, &path);
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(lines) = skill_body_lines(&content) else {
            continue;
        };
        if lines.len() < min_body_lines {
            continue;
        }
        let Some(heading_index) =
            lines.iter().position(|line| line.trim() == CRITICAL_PATH_HEADING)
        else {
            continue;
        };
        let section_lines = critical_path_section_lines(&lines, heading_index);
        let item_count = count_critical_path_items(section_lines);
        if !(min_items..=max_items).contains(&item_count) {
            findings.push(SkillBodyFinding {
                rule_id: RULE_INVALID_CRITICAL_PATH,
                path: rel.clone(),
                message: format!(
                    "Invalid Critical Path: {rel} — expected {min_items}-{max_items} bullets or numbered items, found {item_count}"
                ),
            });
        }
    }
    findings
}

/// CORE-046: step bodies after the `## Critical Path` section must not
/// duplicate Critical Path entries verbatim.
fn check_step_body_duplicates(project_dir: &Path) -> Vec<SkillBodyFinding> {
    let mut findings = Vec::new();
    for path in walk_skill_files(project_dir) {
        let rel = relative_display(project_dir, &path);
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(lines) = skill_body_lines(&content) else {
            continue;
        };
        let Some(cp_start) = lines.iter().position(|line| line.trim() == CRITICAL_PATH_HEADING)
        else {
            continue;
        };
        let cp_end = cp_start
            + 1
            + lines[cp_start + 1..]
                .iter()
                .position(|line| line.starts_with("## "))
                .unwrap_or(lines.len() - cp_start - 1);
        let cp_entries = collect_critical_path_entries(&lines, cp_start + 1, cp_end);
        if cp_entries.is_empty() {
            continue;
        }
        let violations = find_step_body_duplicates(&lines, cp_end, &cp_entries);
        if violations.is_empty() {
            continue;
        }
        let detail: Vec<String> = violations
            .iter()
            .take(3)
            .map(|(line, text)| format!("line {line}: '{}'", truncate(text, 80)))
            .collect();
        let more = if violations.len() > 3 {
            format!(" (+{} more)", violations.len() - 3)
        } else {
            String::new()
        };
        findings.push(SkillBodyFinding {
            rule_id: RULE_STEP_BODY_DUPLICATES,
            path: rel.clone(),
            message: format!(
                "Step body duplicates Critical Path: {rel} — {} match(es): {}{} (Critical Path is the TOC; keep step bodies as short pointers to references)",
                violations.len(),
                detail.join("; "),
                more
            ),
        });
    }
    findings
}

/// CORE-048: `$VAR`s declared in the Arguments section must be referenced
/// in the body, and `$VAR`s used in the body must be defined.
fn check_variable_coverage(project_dir: &Path, builtin_vars: &[String]) -> Vec<SkillBodyFinding> {
    let mut findings = Vec::new();
    for path in walk_skill_files(project_dir) {
        let rel = relative_display(project_dir, &path);
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Some(heading_match) = ARGS_HEADING_RE.find(&content) else {
            continue;
        };
        let heading_idx = heading_match.start();
        let after_heading = &content[heading_match.end()..];
        let section_end =
            after_heading.find("\n## ").map_or(content.len(), |idx| heading_match.end() + idx);
        let args_section = &content[heading_idx..section_end];

        let mut defined = BTreeSet::new();
        let mut used_in_defs = BTreeSet::new();
        for block in TEXT_CODE_BLOCK_RE.captures_iter(args_section) {
            let block_text = block.get(1).map_or("", |m| m.as_str());
            for caps in VAR_DEF_RE.captures_iter(block_text) {
                defined.insert(caps[1].to_string());
            }
            for line in block_text.split('\n') {
                let Some(eq_idx) = line.find('=') else {
                    continue;
                };
                let rhs = &line[eq_idx + 1..];
                for caps in VAR_USE_RE.captures_iter(rhs) {
                    let name = &caps[1];
                    if !is_builtin_var(name, builtin_vars) {
                        used_in_defs.insert(name.to_string());
                    }
                }
            }
        }

        if defined.is_empty() {
            continue;
        }

        let body = &content[section_end..];
        let body_no_fences = FENCE_STRIP_RE.replace_all(body, "").into_owned();
        let used_in_body = collect_var_uses(&body_no_fences, builtin_vars);
        let body_strict = INLINE_CODE_RE.replace_all(&body_no_fences, "").into_owned();
        let used_in_body_strict = collect_var_uses(&body_strict, builtin_vars);

        for var in &defined {
            if !used_in_body.contains(var) && !used_in_defs.contains(var) {
                findings.push(SkillBodyFinding {
                    rule_id: RULE_VARIABLE_COVERAGE,
                    path: rel.clone(),
                    message: format!(
                        "Unused variable: {rel} — ${var} defined but never referenced in body"
                    ),
                });
            }
        }
        for var in &used_in_body_strict {
            if !defined.contains(var)
                && !is_builtin_var(var, builtin_vars)
                && ALL_CAPS_VAR_RE.is_match(var)
            {
                findings.push(SkillBodyFinding {
                    rule_id: RULE_VARIABLE_COVERAGE,
                    path: rel.clone(),
                    message: format!(
                        "Undefined variable: {rel} — ${var} used but not defined in Arguments"
                    ),
                });
            }
        }
    }
    findings
}

fn critical_path_section_lines(lines: &[String], heading_index: usize) -> &[String] {
    let rest = &lines[heading_index + 1..];
    rest.iter().position(|line| line.starts_with("## ")).map_or(rest, |next_h2| &rest[..next_h2])
}

#[derive(Clone, Copy)]
enum CriticalPathMode {
    H3,
    List,
}

fn count_critical_path_items(section_lines: &[String]) -> usize {
    let mut item_count = 0;
    let mut mode: Option<CriticalPathMode> = None;
    for line in section_lines {
        let trimmed = line.trim();
        if mode.is_none() {
            if trimmed.is_empty() {
                continue;
            }
            if line.starts_with("### ") {
                mode = Some(CriticalPathMode::H3);
                item_count += 1;
                continue;
            }
            if LIST_ITEM_RE.is_match(line) {
                mode = Some(CriticalPathMode::List);
                item_count += 1;
            }
            continue;
        }
        match mode {
            Some(CriticalPathMode::H3) if line.starts_with("### ") => item_count += 1,
            Some(CriticalPathMode::H3) | None => {}
            Some(CriticalPathMode::List) => {
                if trimmed.is_empty() {
                    break;
                }
                if LIST_ITEM_RE.is_match(line) {
                    item_count += 1;
                }
            }
        }
    }
    item_count
}

fn collect_critical_path_entries(lines: &[String], start: usize, end: usize) -> BTreeSet<String> {
    let mut cp_entries = BTreeSet::new();
    let mut in_fence = false;
    for line in &lines[start..end] {
        if line.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || !is_list_or_heading_line(line) {
            continue;
        }
        let norm = normalise_entry(line);
        if !norm.is_empty() {
            cp_entries.insert(norm);
        }
    }
    cp_entries
}

fn find_step_body_duplicates(
    lines: &[String], cp_end: usize, cp_entries: &BTreeSet<String>,
) -> Vec<(usize, String)> {
    let mut violations = Vec::new();
    let mut in_fence = false;
    for (i, raw) in lines.iter().enumerate().skip(cp_end) {
        if raw.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || !is_list_or_heading_line(raw) {
            continue;
        }
        let norm = normalise_entry(raw);
        if norm.is_empty() {
            continue;
        }
        if cp_entries.contains(&norm) {
            violations.push((i + 1, raw.trim().to_string()));
        }
    }
    violations
}

fn normalise_entry(text: &str) -> String {
    let mut out = text.to_string();
    out = LIST_PREFIX_RE.replace_all(&out, "").into_owned();
    out = HEADING_PREFIX_RE.replace_all(&out, "").into_owned();
    out = STEP_PREFIX_RE.replace_all(&out, "").into_owned();
    out = WHITESPACE_RE.replace_all(out.trim(), " ").into_owned();
    out.to_lowercase()
}

fn is_list_or_heading_line(line: &str) -> bool {
    LIST_LINE_RE.is_match(line) || line.starts_with("### ") || line.starts_with("#### ")
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max { text.to_string() } else { text[..max].to_string() }
}

fn is_builtin_var(name: &str, builtin_vars: &[String]) -> bool {
    builtin_vars.iter().any(|builtin| builtin == name)
}

fn collect_var_uses(text: &str, builtin_vars: &[String]) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for caps in VAR_USE_RE.captures_iter(text) {
        let name = caps[1].to_string();
        if !is_builtin_var(&name, builtin_vars) {
            out.insert(name);
        }
    }
    out
}

/// Return body lines after the closing frontmatter delimiter, trimming a
/// single leading and trailing blank line.
fn skill_body_lines(content: &str) -> Option<Vec<String>> {
    let block = frontmatter_block(content)?;
    let start = content.find(block)? + block.len();
    let mut lines: Vec<String> = content[start..].split('\n').map(str::to_string).collect();
    if lines.first().is_some_and(String::is_empty) {
        lines.remove(0);
    }
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    Some(lines)
}

fn frontmatter_block(content: &str) -> Option<&str> {
    let rest = content.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

/// Walk every `SKILL.md` under `<project_dir>/plugins`, never following
/// or collecting symlinks.
fn walk_skill_files(project_dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_files(&project_dir.join("plugins"), &mut out);
    out.retain(|path| path.file_name().and_then(|n| n.to_str()) == Some("SKILL.md"));
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_skill(dir: &Path, slug: &str, body: &str) {
        let skill_dir = dir.join("plugins/demo/skills").join(slug);
        std::fs::create_dir_all(&skill_dir).expect("skill dir");
        let content = format!(
            "---\nname: {slug}\ndescription: Fixture. Use when testing skill body checks.\n---\n\n{body}\n"
        );
        std::fs::write(skill_dir.join("SKILL.md"), content).expect("write skill");
    }

    fn padding(count: usize) -> String {
        (0..count).map(|i| format!("padding {i}")).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn invalid_critical_path_flags_wrong_count() {
        let dir = tempfile::tempdir().expect("tempdir");
        let body =
            format!("## Critical Path\n\n1. one\n2. two\n3. three\n4. four\n\n{}", padding(150));
        write_skill(dir.path(), "bad", &body);
        let findings = check_invalid_critical_path(dir.path(), 150, 5, 7);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("found 4"));
    }

    #[test]
    fn variable_coverage_flags_undefined_use() {
        let dir = tempfile::tempdir().expect("tempdir");
        let body = "## Arguments\n\n```text\n$SLICE=<name>\n```\n\n## Steps\n\nValidate $PROJECT for $SLICE before continuing.";
        write_skill(dir.path(), "vars", body);
        let builtins = vec!["ARGUMENTS".to_string(), "HOME".to_string()];
        let findings = check_variable_coverage(dir.path(), &builtins);
        assert_eq!(findings.len(), 1);
        assert!(findings[0].message.contains("Undefined variable"));
        assert!(findings[0].message.contains("$PROJECT"));
    }

    #[test]
    fn clean_short_skill_is_silent() {
        let dir = tempfile::tempdir().expect("tempdir");
        write_skill(dir.path(), "ok", "Just a short body.");
        assert!(check_invalid_critical_path(dir.path(), 150, 5, 7).is_empty());
        assert!(check_step_body_duplicates(dir.path()).is_empty());
    }
}

use std::fs;

use regex::{Regex, RegexBuilder};

use crate::context::Context;
use crate::error::ToolingError;
use crate::finding::{Check, Finding, Location};
use crate::helpers::{relative_display, skill_body_lines, walk_skill_files};

const MAX_BODY_LINES: usize = 200;
const CRITICAL_PATH_MIN_LINES: usize = 150;
const CRITICAL_PATH_HEADING: &str = "## Critical Path";
const MAX_INLINE_JSON_LINES: usize = 30;
const MAX_SECTION_LINES: usize = 45;

const RULE_BODY_LINE_COUNT: &str = "skill.body-line-count";
const RULE_SECTION_LINE_COUNT: &str = "skill.section-line-count";
const RULE_MISSING_CRITICAL_PATH: &str = "skill.missing-critical-path";
const RULE_INVALID_CRITICAL_PATH: &str = "skill.invalid-critical-path";
const RULE_INLINE_JSON_TOO_LONG: &str = "skill.inline-json-too-long";
const RULE_ENVELOPE_JSON_IN_BODY: &str = "skill.envelope-json-in-body";
const RULE_STEP_BODY_DUPLICATES: &str = "skill.step-body-duplicates-critical-path";
const RULE_FRONTMATTER_RESTATEMENT: &str = "skill.frontmatter-restatement";
const RULE_VARIABLE_COVERAGE: &str = "skill.variable-coverage";

/// Skill body must not exceed the global line budget.
pub struct SkillBodyLineCount;

/// Each H2 section must stay within the per-section line budget.
pub struct SkillSectionLineCount;

/// Long skills must include a `## Critical Path` block.
pub struct SkillMissingCriticalPath;

/// Critical Path must list 5–7 steps (list or H3 form).
pub struct SkillInvalidCriticalPath;

/// Inline `json` / `jsonc` fences must not exceed 30 body lines.
pub struct SkillInlineJsonTooLong;

/// SKILL bodies must not embed CLI envelope JSON shapes.
pub struct SkillEnvelopeJsonInBody;

/// Step bodies must not duplicate Critical Path entries verbatim.
pub struct SkillStepBodyDuplicatesCriticalPath;

/// `## Input` restates frontmatter and is forbidden.
pub struct SkillFrontmatterRestatement;

/// `$VAR`s in Arguments must be defined and referenced consistently.
pub struct SkillVariableCoverage;

macro_rules! impl_skill_body_check {
    ($ty:ty, $rule:expr, $body:expr) => {
        impl Check for $ty {
            fn run(&self, ctx: &Context) -> Vec<Finding> {
                $body(ctx).unwrap_or_else(|error| {
                    vec![infrastructure_finding($rule, error)]
                })
            }
        }
    };
}

impl_skill_body_check!(SkillBodyLineCount, RULE_BODY_LINE_COUNT, check_body_line_count);
impl_skill_body_check!(
    SkillSectionLineCount,
    RULE_SECTION_LINE_COUNT,
    check_section_line_counts
);
impl_skill_body_check!(
    SkillMissingCriticalPath,
    RULE_MISSING_CRITICAL_PATH,
    check_missing_critical_path
);
impl_skill_body_check!(
    SkillInvalidCriticalPath,
    RULE_INVALID_CRITICAL_PATH,
    check_invalid_critical_path
);
impl_skill_body_check!(
    SkillInlineJsonTooLong,
    RULE_INLINE_JSON_TOO_LONG,
    check_inline_json_blocks
);
impl_skill_body_check!(
    SkillEnvelopeJsonInBody,
    RULE_ENVELOPE_JSON_IN_BODY,
    check_no_envelope_examples
);
impl_skill_body_check!(
    SkillStepBodyDuplicatesCriticalPath,
    RULE_STEP_BODY_DUPLICATES,
    check_no_step_body_duplicates_critical_path
);
impl_skill_body_check!(
    SkillFrontmatterRestatement,
    RULE_FRONTMATTER_RESTATEMENT,
    check_no_frontmatter_restatement
);
impl_skill_body_check!(
    SkillVariableCoverage,
    RULE_VARIABLE_COVERAGE,
    check_variables
);

fn check_body_line_count(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let root = ctx.framework_root();
    let mut findings = Vec::new();

    for path in walk_skill_files(root)? {
        let rel = relative_display(root, &path);
        let content = fs::read_to_string(&path)?;
        let Some(lines) = skill_body_lines(&content) else {
            continue;
        };

        if lines.len() > MAX_BODY_LINES {
            findings.push(finding(
                RULE_BODY_LINE_COUNT,
                format!(
                    "Skill body too long: {rel} — {} body lines (limit {MAX_BODY_LINES})",
                    lines.len()
                ),
                Some(path),
            ));
        }
    }

    Ok(findings)
}

fn check_section_line_counts(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let root = ctx.framework_root();
    let mut findings = Vec::new();

    for path in walk_skill_files(root)? {
        let rel = relative_display(root, &path);
        let content = fs::read_to_string(&path)?;
        let Some(lines) = skill_body_lines(&content) else {
            continue;
        };

        let h2_indices: Vec<usize> = lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.starts_with("## "))
            .map(|(idx, _)| idx)
            .collect();

        let mut violations = Vec::new();
        for (i, &start) in h2_indices.iter().enumerate() {
            let end = h2_indices
                .get(i + 1)
                .copied()
                .unwrap_or(lines.len());
            let title = lines[start][3..].trim();
            let section_lines = &lines[start + 1..end];
            let cnt = count_section_body_lines(section_lines);
            if cnt > MAX_SECTION_LINES {
                violations.push(format!("'{title}' ({cnt} lines)"));
            }
        }

        if !violations.is_empty() {
            findings.push(finding(
                RULE_SECTION_LINE_COUNT,
                format!(
                    "Skill section too long: {rel} — {} section(s) over {MAX_SECTION_LINES} lines: {} (move depth into references/ and link from the H2)",
                    violations.len(),
                    violations.join(", ")
                ),
                Some(path),
            ));
        }
    }

    Ok(findings)
}

fn count_section_body_lines(section_lines: &[String]) -> usize {
    let mut count = 0;
    let mut in_fence = false;
    for line in section_lines {
        if line.starts_with("```") {
            in_fence = !in_fence;
            count += 1;
            continue;
        }
        if in_fence {
            count += 1;
            continue;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("<!--") && trimmed.ends_with("-->") {
            continue;
        }
        count += 1;
    }
    count
}

fn check_missing_critical_path(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let root = ctx.framework_root();
    let mut findings = Vec::new();

    for path in walk_skill_files(root)? {
        let rel = relative_display(root, &path);
        let content = fs::read_to_string(&path)?;
        let Some(lines) = skill_body_lines(&content) else {
            continue;
        };

        if lines.len() < CRITICAL_PATH_MIN_LINES {
            continue;
        }

        let has_heading = lines
            .iter()
            .any(|line| line.trim() == CRITICAL_PATH_HEADING);
        if !has_heading {
            findings.push(finding(
                RULE_MISSING_CRITICAL_PATH,
                format!(
                    "Missing Critical Path: {rel} — {} body lines requires '{CRITICAL_PATH_HEADING}'",
                    lines.len()
                ),
                Some(path),
            ));
        }
    }

    Ok(findings)
}

fn check_invalid_critical_path(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let list_item_re = Regex::new(r"^(?:\d+\.|-)\s+\S").expect("list item pattern");
    let root = ctx.framework_root();
    let mut findings = Vec::new();

    for path in walk_skill_files(root)? {
        let rel = relative_display(root, &path);
        let content = fs::read_to_string(&path)?;
        let Some(lines) = skill_body_lines(&content) else {
            continue;
        };

        if lines.len() < CRITICAL_PATH_MIN_LINES {
            continue;
        }

        let Some(heading_index) = lines
            .iter()
            .position(|line| line.trim() == CRITICAL_PATH_HEADING)
        else {
            continue;
        };

        let section_lines = critical_path_section_lines(&lines, heading_index);
        let item_count = count_critical_path_items(&section_lines, &list_item_re);

        if !(5..=7).contains(&item_count) {
            findings.push(finding(
                RULE_INVALID_CRITICAL_PATH,
                format!(
                    "Invalid Critical Path: {rel} — expected 5-7 bullets or numbered items, found {item_count}"
                ),
                Some(path),
            ));
        }
    }

    Ok(findings)
}

fn critical_path_section_lines(lines: &[String], heading_index: usize) -> &[String] {
    let rest = &lines[heading_index + 1..];
    if let Some(next_h2) = rest.iter().position(|line| line.starts_with("## ")) {
        &rest[..next_h2]
    } else {
        rest
    }
}

fn count_critical_path_items(section_lines: &[String], list_item_re: &Regex) -> usize {
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
            if list_item_re.is_match(line) {
                mode = Some(CriticalPathMode::List);
                item_count += 1;
            }
            continue;
        }
        match mode {
            Some(CriticalPathMode::H3) => {
                if line.starts_with("### ") {
                    item_count += 1;
                }
            }
            Some(CriticalPathMode::List) => {
                if trimmed.is_empty() {
                    break;
                }
                if list_item_re.is_match(line) {
                    item_count += 1;
                }
            }
            None => {}
        }
    }

    item_count
}

#[derive(Clone, Copy)]
enum CriticalPathMode {
    H3,
    List,
}

fn check_inline_json_blocks(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let fence_open_re = Regex::new(r"^```(json|jsonc)\b").expect("json fence pattern");
    let root = ctx.framework_root();
    let mut findings = Vec::new();

    for path in walk_skill_files(root)? {
        let rel = relative_display(root, &path);
        let content = fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.split('\n').collect();

        let mut in_block = false;
        let mut block_start = 0usize;
        let mut block_length = 0usize;

        for (i, line) in lines.iter().enumerate() {
            if !in_block {
                if fence_open_re.is_match(line) {
                    in_block = true;
                    block_start = i + 1;
                    block_length = 0;
                }
                continue;
            }
            if line.starts_with("```") {
                if block_length > MAX_INLINE_JSON_LINES {
                    findings.push(finding(
                        RULE_INLINE_JSON_TOO_LONG,
                        format!(
                            "Inline JSON too long: {rel}:{} — {block_length} body lines (limit {MAX_INLINE_JSON_LINES}); move large output shapes to docs/reference/cli-output-shapes.md and link to them",
                            block_start + 1
                        ),
                        Some(path.clone()),
                    ));
                }
                in_block = false;
                continue;
            }
            block_length += 1;
        }
    }

    Ok(findings)
}

fn check_no_envelope_examples(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let fence_open_re = Regex::new(r"^\s*(`{3,})(json|jsonc)\b").expect("json fence pattern");
    let root = ctx.framework_root();
    let mut findings = Vec::new();

    for path in walk_skill_files(root)? {
        let rel = relative_display(root, &path);
        let content = fs::read_to_string(&path)?;
        let lines: Vec<&str> = content.split('\n').collect();

        let mut in_block = false;
        let mut block_start = 0usize;
        let mut block_body: Vec<String> = Vec::new();
        let mut open_fence: Option<String> = None;
        let mut violations = Vec::new();

        for (i, line) in lines.iter().enumerate() {
            if !in_block {
                if let Some(caps) = fence_open_re.captures(line) {
                    in_block = true;
                    open_fence = Some(caps[1].to_string());
                    block_start = i + 1;
                    block_body.clear();
                }
                continue;
            }

            let fence = open_fence.as_deref().unwrap_or("```");
            let close_re = RegexBuilder::new(&format!(r"^\s*{fence}\s*$"))
                .build()
                .expect("fence close pattern");
            if close_re.is_match(line) {
                if is_envelope_body(&block_body) {
                    violations.push(block_start + 1);
                }
                in_block = false;
                open_fence = None;
                block_body.clear();
                continue;
            }
            block_body.push((*line).to_string());
        }

        if !violations.is_empty() {
            let where_str = violations
                .iter()
                .map(|n| format!("line {n}"))
                .collect::<Vec<_>>()
                .join(", ");
            findings.push(finding(
                RULE_ENVELOPE_JSON_IN_BODY,
                format!(
                    "Envelope JSON in skill body: {rel} — {} block(s) at {where_str} (link to docs/reference/cli-output-shapes.md instead of embedding the envelope shape)",
                    violations.len()
                ),
                Some(path),
            ));
        }
    }

    Ok(findings)
}

fn is_envelope_body(body: &[String]) -> bool {
    let text = body.join("\n");
    let envelope_version =
        Regex::new(r#""envelope[-_]version"\s*:"#).expect("envelope version pattern");
    if envelope_version.is_match(&text) {
        return true;
    }
    let has_ok = Regex::new(r#""ok"\s*:\s*(true|false)\b"#)
        .expect("ok pattern")
        .is_match(&text);
    let has_data = Regex::new(r#""data"\s*:"#).expect("data pattern").is_match(&text);
    let has_error = Regex::new(r#""error"\s*:\s*\{"#)
        .expect("error pattern")
        .is_match(&text);
    has_ok && (has_data || has_error)
}

fn check_no_step_body_duplicates_critical_path(
    ctx: &Context,
) -> Result<Vec<Finding>, ToolingError> {
    let root = ctx.framework_root();
    let mut findings = Vec::new();

    for path in walk_skill_files(root)? {
        let rel = relative_display(root, &path);
        let content = fs::read_to_string(&path)?;
        let Some(lines) = skill_body_lines(&content) else {
            continue;
        };

        let Some(cp_start) = lines
            .iter()
            .position(|line| line.trim() == "## Critical Path")
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
        findings.push(finding(
            RULE_STEP_BODY_DUPLICATES,
            format!(
                "Step body duplicates Critical Path: {rel} — {} match(es): {}{} (Critical Path is the TOC; keep step bodies as short pointers to references)",
                violations.len(),
                detail.join("; "),
                more
            ),
            Some(path),
        ));
    }

    Ok(findings)
}

fn collect_critical_path_entries(
    lines: &[String],
    start: usize,
    end: usize,
) -> std::collections::HashSet<String> {
    let mut cp_entries = std::collections::HashSet::new();
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
    lines: &[String],
    cp_end: usize,
    cp_entries: &std::collections::HashSet<String>,
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
    let step_prefix = Regex::new(r"^Step\s+\d+\s*[:.\-]\s*").expect("step prefix");
    let list_prefix = Regex::new(r"^(?:\d+\.|-|\*)\s+").expect("list prefix");
    let heading_prefix = Regex::new(r"^#{2,4}\s+").expect("heading prefix");
    let whitespace = Regex::new(r"\s+").expect("whitespace");

    let mut out = text.to_string();
    out = list_prefix.replace_all(&out, "").into_owned();
    out = heading_prefix.replace_all(&out, "").into_owned();
    out = step_prefix.replace_all(&out, "").into_owned();
    out = whitespace.replace_all(out.trim(), " ").into_owned();
    out.to_lowercase()
}

fn is_list_or_heading_line(line: &str) -> bool {
    Regex::new(r"^(?:\d+\.|-|\*)\s+\S")
        .expect("list line")
        .is_match(line)
        || line.starts_with("### ")
        || line.starts_with("#### ")
}

fn check_no_frontmatter_restatement(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let root = ctx.framework_root();
    let mut findings = Vec::new();

    for path in walk_skill_files(root)? {
        let rel = relative_display(root, &path);
        let content = fs::read_to_string(&path)?;
        let Some(lines) = skill_body_lines(&content) else {
            continue;
        };

        if let Some(idx) = lines.iter().position(|line| line.trim() == "## Input") {
            findings.push(finding(
                RULE_FRONTMATTER_RESTATEMENT,
                format!(
                    "Frontmatter restated in skill body: {rel}:{} — '## Input' restates the argument-hint already rendered on every invocation; drop the H2 (the inference / prompt instruction belongs in Critical Path step 1)",
                    idx + 1
                ),
                Some(path),
            ));
        }
    }

    Ok(findings)
}

fn check_variables(ctx: &Context) -> Result<Vec<Finding>, ToolingError> {
    let def_re = Regex::new(r"(?m)^\$([A-Z_][A-Z_0-9]*)\s*=")
        .expect("def pattern");
    let use_re = Regex::new(r"\$([A-Z_][A-Z_0-9]*)").expect("use pattern");
    let args_heading_re =
        Regex::new(r"(?m)^## (?:Derived )?Arguments").expect("args heading");
    let code_block_re = Regex::new(r"```text\n([\s\S]*?)```").expect("text code block");
    let fence_re = Regex::new(r"```[\s\S]*?```").expect("fence strip");
    let inline_code_re = Regex::new(r"`[^`]+`").expect("inline code strip");
    let all_caps_var = Regex::new(r"^[A-Z][A-Z_]+$").expect("all caps var");

    let root = ctx.framework_root();
    let mut findings = Vec::new();

    for path in walk_skill_files(root)? {
        let rel = relative_display(root, &path);
        let content = fs::read_to_string(&path)?;

        let Some(heading_match) = args_heading_re.find(&content) else {
            continue;
        };
        let heading_idx = heading_match.start();
        let after_heading = &content[heading_match.end()..];
        let section_end = after_heading
            .find("\n## ")
            .map(|idx| heading_match.end() + idx)
            .unwrap_or(content.len());
        let args_section = &content[heading_idx..section_end];

        let mut defined = std::collections::HashSet::new();
        let mut used_in_defs = std::collections::HashSet::new();

        for block in code_block_re.captures_iter(args_section) {
            let block_text = block.get(1).map(|m| m.as_str()).unwrap_or("");
            for caps in def_re.captures_iter(block_text) {
                defined.insert(caps[1].to_string());
            }
            for line in block_text.split('\n') {
                let Some(eq_idx) = line.find('=') else {
                    continue;
                };
                let rhs = &line[eq_idx + 1..];
                for caps in use_re.captures_iter(rhs) {
                    let name = &caps[1];
                    if !is_builtin_var(name) {
                        used_in_defs.insert(name.to_string());
                    }
                }
            }
        }

        if defined.is_empty() {
            continue;
        }

        let body = &content[section_end..];
        let body_no_fences = fence_re.replace_all(body, "").into_owned();

        let used_in_body = collect_var_uses(&body_no_fences, &use_re);
        let body_strict = inline_code_re
            .replace_all(&body_no_fences, "")
            .into_owned();
        let used_in_body_strict = collect_var_uses(&body_strict, &use_re);

        for var in &defined {
            if !used_in_body.contains(var) && !used_in_defs.contains(var) {
                findings.push(finding(
                    RULE_VARIABLE_COVERAGE,
                    format!("Unused variable: {rel} — ${var} defined but never referenced in body"),
                    Some(path.clone()),
                ));
            }
        }
        for var in &used_in_body_strict {
            if !defined.contains(var) && !is_builtin_var(var) && all_caps_var.is_match(var) {
                findings.push(finding(
                    RULE_VARIABLE_COVERAGE,
                    format!("Undefined variable: {rel} — ${var} used but not defined in Arguments"),
                    Some(path.clone()),
                ));
            }
        }
    }

    Ok(findings)
}

fn is_builtin_var(name: &str) -> bool {
    matches!(name, "ARGUMENTS" | "HOME")
}

fn collect_var_uses(text: &str, use_re: &Regex) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    for caps in use_re.captures_iter(text) {
        let name = caps[1].to_string();
        if !is_builtin_var(&name) {
            out.insert(name);
        }
    }
    out
}

fn truncate(text: &str, max: usize) -> String {
    if text.len() <= max {
        text.to_string()
    } else {
        text[..max].to_string()
    }
}

fn finding(rule_id: &'static str, message: String, path: Option<std::path::PathBuf>) -> Finding {
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

fn infrastructure_finding(rule_id: &'static str, error: ToolingError) -> Finding {
    Finding {
        rule_id,
        message: error.to_string(),
        location: None,
    }
}

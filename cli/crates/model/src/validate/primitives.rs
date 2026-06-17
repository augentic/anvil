//! Structural primitives — the small, unit-testable checks that named
//! rules compose. Each helper is `pub` (crate-internal via the private
//! `primitives` module) and side-effect free
//! apart from `specs_dir` reads in two helpers.

use std::collections::HashSet;
use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

use crate::spec::ParsedSpec;
use crate::task::Progress;

// ---------------------------------------------------------------------------
// Compiled regexes (constructed once, on first use). Mirrors the pattern in
// `crate::task` so the model crate is uniformly OnceLock-backed for literal
// patterns. The dynamic `ids_match_pattern` accessor stays inline because its
// pattern is caller-supplied.
// ---------------------------------------------------------------------------

fn checkbox_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^\s*-\s+\[( |x|X)\]\s+\d+(?:\.\d+)*\s+").expect("checkbox regex is valid")
    })
}

/// Unanchored *scanner* over the canonical `REQ_ID_PATTERN` grammar: finds
/// `REQ-NNN` references embedded in design prose. Distinct from the
/// full-string `crate::spec::is_req_id` predicate — anchoring it
/// would stop it matching references inside surrounding text. The pattern
/// body is sourced from `REQ_ID_PATTERN` (anchors stripped) so the scanner
/// and the predicate share one grammar definition.
fn req_id_ref_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        let body = crate::spec::REQ_ID_PATTERN.trim_start_matches('^').trim_end_matches('$');
        Regex::new(body).expect("req id scanner regex is valid")
    })
}

/// Return `true` when `heading` appears AND at least one non-empty,
/// non-whitespace line follows it before the next `##`-or-higher heading.
/// Blank lines between the heading and prose are fine.
pub fn has_content_after_heading(content: &str, heading: &str) -> bool {
    let mut lines = content.lines();
    while let Some(line) = lines.next() {
        if line.trim_end() != heading {
            continue;
        }
        // We've seen the heading; look ahead for prose.
        for follow in lines.by_ref() {
            let trimmed = follow.trim();
            if trimmed.is_empty() {
                continue;
            }
            if is_next_section_boundary(follow, heading) {
                // Hit a sibling/ancestor heading before finding prose.
                return false;
            }
            return true;
        }
        return false;
    }
    false
}

/// A heading line `##` or deeper that isn't the *same* heading we're
/// parsing is treated as the end of the current section. We compare levels
/// by counting leading `#`s: once we see a heading whose level is ≤ the
/// level of `current`, we've left the section.
fn is_next_section_boundary(line: &str, current: &str) -> bool {
    let current_level = leading_hash_count(current);
    let candidate_level = leading_hash_count(line.trim_start());
    // Not a heading at all.
    if candidate_level == 0 {
        return false;
    }
    candidate_level <= current_level
}

fn leading_hash_count(line: &str) -> usize {
    let trimmed = line.trim_start();
    let count = trimmed.chars().take_while(|c| *c == '#').count();
    // Require that the '#'s are followed by a space (or end-of-line) for
    // this to count as a heading; otherwise `#hashtag` false-matches.
    if count == 0 {
        return 0;
    }
    let rest = &trimmed[count..];
    if rest.is_empty() || rest.starts_with(' ') || rest.starts_with('\t') { count } else { 0 }
}

pub fn all_requirements_have_scenarios(spec: &ParsedSpec) -> bool {
    spec.requirements.iter().all(|r| !r.scenarios.is_empty())
}

pub fn all_requirements_have_ids(spec: &ParsedSpec) -> bool {
    spec.requirements.iter().all(|r| !r.id.is_empty())
}

/// Compile `pattern` as a regex and return `true` iff every requirement's
/// `id` fully matches. Invalid patterns (programmer error) return `false`.
pub fn ids_match_pattern(spec: &ParsedSpec, pattern: &str) -> bool {
    let Ok(re) = Regex::new(pattern) else {
        return false;
    };
    spec.requirements.iter().all(|r| {
        let Some(m) = re.find(&r.id) else {
            return false;
        };
        m.start() == 0 && m.end() == r.id.len()
    })
}

/// `true` iff every line starting with `-` in `content` was recognised by
/// the task parser (i.e. it's a `- [ ] X.Y …` checkbox). Non-checkbox
/// bullets like `- bare item` cause a `false` return.
///
/// Also returns `false` if the parsed total disagrees with the recognised
/// count (defensive — shouldn't happen by construction).
pub fn all_tasks_use_checkbox(tasks: &Progress, content: &str) -> bool {
    if tasks.total != tasks.tasks.len() {
        return false;
    }
    for line in content.lines() {
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix('-') else {
            continue;
        };
        // Require at least one whitespace between `-` and a non-space payload
        // (mirrors the previous `^\s*-\s+\S` regex). Plain `-` separators or
        // `-foo` are not bullets.
        let after_ws = rest.trim_start_matches(|c: char| c.is_whitespace());
        if after_ws.len() == rest.len() || after_ws.is_empty() {
            continue;
        }
        if !checkbox_re().is_match(line) {
            return false;
        }
    }
    true
}

pub fn tasks_grouped_under_headings(tasks: &Progress) -> bool {
    tasks.tasks.iter().all(|t| !t.group.is_empty())
}

/// Return `true` iff every domain entry listed under the proposal's
/// `## Domains` section has a matching `specs/<name>/spec.md` on disk.
/// If the section is absent or empty, returns `true` — the sibling
/// `has-content-after-heading` rule is responsible for that case.
pub fn proposal_deliverables_have_specs(proposal: &str, specs_dir: &Path) -> bool {
    let entries = extract_deliverables(proposal, "## Domains");
    if entries.is_empty() {
        return true;
    }
    for name in entries {
        let spec_path = specs_dir.join(&name).join("spec.md");
        if !spec_path.exists() {
            return false;
        }
    }
    true
}

/// Parse the proposal for entries under `heading`. Accepts `- name`,
/// `` - `name` ``, or sub-headings (`### New Domains` / `### Modified Domains`)
/// whose bullets are in turn parsed. Placeholder tokens (values that look
/// like HTML comments) are skipped.
fn extract_deliverables(proposal: &str, heading: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut in_section = false;
    let mut section_level = 0_usize;
    for line in proposal.lines() {
        let trimmed_end = line.trim_end();
        if !in_section {
            if trimmed_end == heading {
                in_section = true;
                section_level = leading_hash_count(heading);
            }
            continue;
        }
        // Leaving the section on a sibling/ancestor heading.
        let level = leading_hash_count(line.trim_start());
        if level > 0 && level <= section_level {
            break;
        }
        let content = line.trim();
        let Some(rest) = content.strip_prefix("- ") else {
            continue;
        };
        let rest = rest.trim();
        if rest.is_empty() {
            continue;
        }
        // Skip comment-shaped placeholder lines.
        if rest.starts_with("<!--") {
            continue;
        }
        // Accept either `- name`, `- `name`` (backtick-wrapped), or
        // `- **name**`. Split on whitespace and pick the first token,
        // stripping decorations.
        let first_token = rest.split_whitespace().next().unwrap_or("");
        let cleaned =
            first_token.trim_matches(|c: char| c == '`' || c == '*' || c == ':' || c == ',').trim();
        if cleaned.is_empty() {
            continue;
        }
        out.push(cleaned.to_string());
    }
    out
}

/// Extract the `$ref` target path from a YAML line like
/// `$ref: "../schemas/user.yaml"`. Returns `None` when the line is not a
/// `$ref:` entry or the value is empty/non-file (fragment-only or URL).
pub fn extract_ref(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix("$ref:")?;
    let value = rest.trim().trim_matches('"').trim_matches('\'');
    if value.is_empty() || value.starts_with('#') || value.contains("://") {
        return None;
    }
    let path_part = value.split('#').next().unwrap_or(value);
    if path_part.is_empty() {
        return None;
    }
    Some(path_part)
}

/// Match `REQ-XXX` IDs in the design doc; return `true` iff each is present
/// in at least one `specs/*/spec.md` under `specs_dir`. Returns `true` if
/// no references are found.
pub fn design_references_exist(design: &str, specs_dir: &Path) -> bool {
    let refs: HashSet<String> =
        req_id_ref_re().find_iter(design).map(|m| m.as_str().to_string()).collect();
    if refs.is_empty() {
        return true;
    }
    let Ok(dir_iter) = std::fs::read_dir(specs_dir) else {
        return false;
    };
    let mut spec_bodies: Vec<String> = Vec::new();
    for entry in dir_iter.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let spec_path = path.join("spec.md");
        if let Ok(contents) = std::fs::read_to_string(&spec_path) {
            spec_bodies.push(contents);
        }
    }
    refs.iter().all(|needle| spec_bodies.iter().any(|body| body.contains(needle)))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn tmp() -> TempDir {
        tempfile::tempdir().expect("tempdir")
    }

    #[test]
    fn content_after_heading_prose() {
        let ok = "## Why\n\nbecause the problem exists\n\n## Next\n";
        assert!(has_content_after_heading(ok, "## Why"));
        let empty = "## Why\n\n## Next\nstuff\n";
        assert!(!has_content_after_heading(empty, "## Why"));
        let subheading_only = "## Why\n### Child\n\n## Next\n";
        // `### Child` is a child heading (deeper), prose-ish enough.
        assert!(has_content_after_heading(subheading_only, "## Why"));
        assert!(!has_content_after_heading("no such heading", "## Why"));
    }

    #[test]
    fn requirements_need_scenarios() {
        let ok = crate::spec::parse_baseline(
            "### Requirement: Thing\n\nID: REQ-001\n\n#### Scenario: Happy\n- WHEN foo\n- THEN bar\n",
        );
        assert!(all_requirements_have_scenarios(&ok));
        let bad =
            crate::spec::parse_baseline("### Requirement: Thing\n\nID: REQ-001\n\nno scenario\n");
        assert!(!all_requirements_have_scenarios(&bad));
    }

    #[test]
    fn requirements_need_ids() {
        let ok = crate::spec::parse_baseline(
            "### Requirement: Thing\n\nID: REQ-001\n\n#### Scenario: Happy\n",
        );
        assert!(all_requirements_have_ids(&ok));
        let bad = crate::spec::parse_baseline("### Requirement: Thing\n\n#### Scenario: Happy\n");
        assert!(!all_requirements_have_ids(&bad));
    }

    #[test]
    fn req_id_pattern() {
        let ok = crate::spec::parse_baseline(
            "### Requirement: Thing\n\nID: REQ-001\n\n#### Scenario: Happy\n",
        );
        assert!(ids_match_pattern(&ok, crate::spec::REQ_ID_PATTERN));
        let bad = crate::spec::parse_baseline(
            "### Requirement: Thing\n\nID: REQ-1\n\n#### Scenario: Happy\n",
        );
        assert!(!ids_match_pattern(&bad, crate::spec::REQ_ID_PATTERN));
    }

    #[test]
    fn checkbox_rejects_bare_bullets() {
        let ok = "## 1. Setup\n- [ ] 1.1 Do thing\n- [ ] 1.2 Do other\n";
        let progress = crate::task::parse_tasks(ok);
        assert!(all_tasks_use_checkbox(&progress, ok));
        let bad = "## 1. Setup\n- [ ] 1.1 Do thing\n- bare bullet\n";
        let progress = crate::task::parse_tasks(bad);
        assert!(!all_tasks_use_checkbox(&progress, bad));
    }

    #[test]
    fn tasks_require_group_headings() {
        let ok = "## 1. Setup\n- [ ] 1.1 Do thing\n";
        let progress = crate::task::parse_tasks(ok);
        assert!(tasks_grouped_under_headings(&progress));
        let bad = "- [ ] 1.1 Do thing\n";
        let progress = crate::task::parse_tasks(bad);
        assert!(!tasks_grouped_under_headings(&progress));
    }

    #[test]
    fn deliverable_specs_on_disk() {
        let dir = tmp();
        let specs = dir.path().join("specs");
        fs::create_dir_all(specs.join("login")).unwrap();
        fs::write(specs.join("login").join("spec.md"), "# Login\n").unwrap();

        let ok_proposal = "## Domains\n\n- login\n";
        assert!(proposal_deliverables_have_specs(ok_proposal, &specs));

        let missing = "## Domains\n\n- login\n- missing\n";
        assert!(!proposal_deliverables_have_specs(missing, &specs));

        // Absent section → true (the has-content rule handles this case).
        let absent = "## Why\n\nbecause\n";
        assert!(proposal_deliverables_have_specs(absent, &specs));
    }

    #[test]
    fn deliverables_backticked_names() {
        let dir = tmp();
        let specs = dir.path().join("specs");
        fs::create_dir_all(specs.join("user-auth")).unwrap();
        fs::write(specs.join("user-auth").join("spec.md"), "# Login\n").unwrap();
        let proposal = "## Domains\n\n### New Domains\n\n- `user-auth`\n";
        assert!(proposal_deliverables_have_specs(proposal, &specs));
    }

    #[test]
    fn extract_ref_double_quoted() {
        assert_eq!(
            extract_ref(r#"              $ref: "../schemas/user.yaml""#),
            Some("../schemas/user.yaml")
        );
    }

    #[test]
    fn extract_ref_single_quoted() {
        assert_eq!(extract_ref("    $ref: '../schemas/user.yaml'"), Some("../schemas/user.yaml"));
    }

    #[test]
    fn extract_ref_unquoted() {
        assert_eq!(extract_ref("  $ref: ../schemas/user.yaml"), Some("../schemas/user.yaml"));
    }

    #[test]
    fn extract_ref_fragment() {
        assert_eq!(extract_ref("$ref: \"#/components/schemas/Error\""), None);
    }

    #[test]
    fn extract_ref_http_url() {
        assert_eq!(extract_ref("$ref: \"https://example.com/schemas/user.yaml\""), None);
    }

    #[test]
    fn extract_ref_non_ref_line() {
        assert_eq!(extract_ref("  type: string"), None);
        assert_eq!(extract_ref("  description: a $ref example"), None);
    }

    #[test]
    fn design_refs_backed_by_specs() {
        let dir = tmp();
        let specs = dir.path().join("specs");
        fs::create_dir_all(specs.join("a")).unwrap();
        fs::write(specs.join("a").join("spec.md"), "### Requirement: X\nID: REQ-001\n").unwrap();

        let ok = "See REQ-001 in the spec.";
        assert!(design_references_exist(ok, &specs));

        let missing = "See REQ-999 in the spec.";
        assert!(!design_references_exist(missing, &specs));

        let none = "No references in this doc.";
        assert!(design_references_exist(none, &specs));
    }
}

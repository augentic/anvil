//! `specify-ignore` directive extractor.
//!
//! Recognises one grammar — `specify-ignore: <RULE-ID> — <rationale>`
//! — inside the closed comment-style list:
//!
//! - C-family (`rust`, `swift`, `kotlin`, `typescript`, `javascript`):
//!   `// specify-ignore: …` and `/* specify-ignore: … */`.
//! - Hash (`python`, `yaml`, `toml`): `# specify-ignore: …`.
//! - HTML (`markdown`): `<!-- specify-ignore: … -->`.
//! - SQL/Lua (`sql`): `-- specify-ignore: …`.
//!
//! Files whose inferred language sits outside the list are skipped
//! without falling back to heuristics; binary files are skipped too.
//!
//! Malformed directives (missing rationale, no separator, or empty
//! rationale) are still emitted with `rationale = None` so the
//! directive-validation pass can synthesise `UNI-022`. The
//! length-check for rationales
//! shorter than 16 characters is also the validation pass's job; this
//! extractor captures whatever rationale text is present.
//!
//! `target_line` follows the ignore-directive scope rules:
//!
//! - Block-leading directives (the comment is the first non-whitespace
//!   on the line) target the next non-blank, non-comment line.
//!   Multiple consecutive block-leading directives compose: each
//!   points at the same eventual code line.
//! - Inline trailing directives (the comment follows code on the same
//!   line) target the line they live on.
//! - When no following non-blank, non-comment line exists,
//!   `target_line` is set to one past the file's last line so the
//!   validation pass treats the directive as orphaned.

mod parse;

use super::files::DiscoveredFile;
use crate::lint::{FileKind, IgnoreDirective};

/// Closed set of comment families the extractor recognises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Family {
    /// `// …` and `/* … */`.
    C,
    /// `# …`.
    Hash,
    /// `<!-- … -->`.
    Html,
    /// `-- …`.
    SqlLua,
}

/// Pick a comment family from the indexer-inferred language token,
/// or `None` for languages outside the closed list.
fn family_for(language: &str) -> Option<Family> {
    match language {
        "rust" | "swift" | "kotlin" | "typescript" | "javascript" => Some(Family::C),
        "python" | "yaml" | "toml" => Some(Family::Hash),
        "markdown" => Some(Family::Html),
        "sql" => Some(Family::SqlLua),
        _ => None,
    }
}

/// Extract every `specify-ignore` directive from `file`. Returns an
/// empty vector when the file is binary or its language sits outside
/// the closed comment-style list.
#[must_use]
pub fn extract(file: &DiscoveredFile) -> Vec<IgnoreDirective> {
    if file.kind != FileKind::Text {
        return Vec::new();
    }
    let Some(family) = file.language.as_deref().and_then(family_for) else {
        return Vec::new();
    };

    let text = file.text();
    let lines: Vec<&str> = text.split('\n').collect();
    let total_lines = u32::try_from(lines.len()).unwrap_or(u32::MAX);

    let parsed: Vec<Option<parse::Parsed>> =
        lines.iter().map(|line| parse::parse_line(line, family)).collect();

    let mut out: Vec<IgnoreDirective> = Vec::new();
    for (idx, slot) in parsed.iter().enumerate() {
        let Some(p) = slot else { continue };
        let line_no = u32::try_from(idx + 1).unwrap_or(u32::MAX);
        let target_line =
            if p.is_trailing { line_no } else { find_target(&lines, idx, family, total_lines) };
        out.push(IgnoreDirective {
            path: file.relative.clone(),
            line: line_no,
            rule_id: p.rule_id.clone(),
            rationale: p.rationale.clone(),
            target_line,
            raw: p.raw.clone(),
        });
    }
    out
}

/// Walk forward from the line at `idx` and return the 1-based line
/// of the next non-blank, non-comment-only line. When no such line
/// exists, return `total_lines + 1` so the validation pass can
/// detect that the directive sits past EOF.
fn find_target(lines: &[&str], idx: usize, family: Family, total_lines: u32) -> u32 {
    for (offset, line) in lines.iter().enumerate().skip(idx + 1) {
        if line.trim().is_empty() {
            continue;
        }
        if is_comment_only(line, family) {
            continue;
        }
        return u32::try_from(offset + 1).unwrap_or(u32::MAX);
    }
    total_lines.saturating_add(1)
}

/// Per-family leading-delimiter check used to skip comment-only
/// lines while walking forward to the directive's target line.
fn is_comment_only(line: &str, family: Family) -> bool {
    let trimmed = line.trim_start();
    if trimmed.is_empty() {
        return false;
    }
    match family {
        Family::C => trimmed.starts_with("//") || trimmed.starts_with("/*"),
        Family::Hash => trimmed.starts_with('#'),
        Family::Html => trimmed.starts_with("<!--"),
        Family::SqlLua => trimmed.starts_with("--"),
    }
}

#[cfg(test)]
mod tests;

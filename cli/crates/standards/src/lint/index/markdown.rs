//! Markdown structure + link scan per the `WorkspaceModel` entity families.
//!
//! Two byte-stable passes over each markdown file's text:
//!
//! - [`extract_sections`] records ATX-style headings (`#` … `######`),
//!   tracks each heading's `(line_start, line_end, body_line_count)`,
//!   and closes a section when the next same-or-shallower heading
//!   begins (or at EOF).
//! - [`extract_links`] scans for `[label](target)` markdown links.
//!
//! Both passes share the same fence and HTML-comment state machine:
//! a heading or a link that lands inside a triple-backtick or `~~~`
//! fence or inside a `<!-- … -->` block is skipped. The §"Stability"
//! rule requires byte-stable output across runs, not perfect markdown
//! semantics — the scanner trades `CommonMark` edge cases for a
//! single flat pass with no parser dependency.

use std::borrow::Cow;
use std::sync::LazyLock;

use regex::Regex;

use super::files::DiscoveredFile;
use crate::lint::{FencedBlock, MarkdownLink, MarkdownSection};

static FENCE_OPEN_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^\s*(`{3,}|~{3,})(\S*)").expect("fence open regex"));

/// Extract ATX-style section facts from a markdown file. Non-markdown
/// files return an empty vector.
#[must_use]
pub fn extract_sections(file: &DiscoveredFile) -> Vec<MarkdownSection> {
    if file.language.as_deref() != Some("markdown") {
        return Vec::new();
    }
    let text = file.text();
    let lines: Vec<&str> = text.split('\n').collect();
    let total_lines = u32::try_from(lines.len()).unwrap_or(u32::MAX);

    let mut state = ScanState::default();
    let mut headings: Vec<(u8, String, u32)> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let line_no = u32::try_from(idx + 1).unwrap_or(u32::MAX);
        let Some(scanned) = state.process(line) else {
            continue;
        };
        if let Some((level, title)) = parse_heading(scanned.as_ref()) {
            headings.push((level, title, line_no));
        }
    }

    let mut sections: Vec<MarkdownSection> = Vec::with_capacity(headings.len());
    for (i, (level, title, line_start)) in headings.iter().enumerate() {
        let line_end = headings
            .iter()
            .skip(i + 1)
            .find(|(next_level, _, _)| next_level <= level)
            .map_or(total_lines, |(_, _, next_start)| next_start.saturating_sub(1));
        let line_end = line_end.max(*line_start);
        let body_line_count = line_end.saturating_sub(*line_start);
        sections.push(MarkdownSection {
            path: file.relative.clone(),
            level: *level,
            title: title.clone(),
            line_start: *line_start,
            line_end,
            body_line_count,
        });
    }
    sections
}

/// Extract `[label](target)` link facts from a markdown file.
/// `resolves` is left at `None`; the umbrella's sequential pass fills
/// it in by checking each `to_raw` against the discovered file set.
#[must_use]
pub fn extract_links(file: &DiscoveredFile) -> Vec<MarkdownLink> {
    if file.language.as_deref() != Some("markdown") {
        return Vec::new();
    }
    let text = file.text();
    let mut state = ScanState::default();
    let mut links: Vec<MarkdownLink> = Vec::new();
    for (idx, line) in text.split('\n').enumerate() {
        let line_no = u32::try_from(idx + 1).unwrap_or(u32::MAX);
        let Some(scanned) = state.process(line) else {
            continue;
        };
        scan_line_for_links(scanned.as_ref(), &file.relative, line_no, &mut links);
    }
    links
}

#[derive(Default)]
struct ScanState {
    in_fence: bool,
    fence_marker: Option<String>,
    in_comment: bool,
}

impl ScanState {
    /// Advance the state with `line` and return the bytes that
    /// section / link scanners should consume:
    ///
    /// - `None` when the line is entirely inside a fence, entirely
    ///   inside an HTML comment, or is itself the opening/closing
    ///   delimiter of a fence or multi-line comment.
    /// - `Some(Cow::Borrowed(line))` when the line is plain markdown
    ///   with no inline HTML comment to strip.
    /// - `Some(Cow::Owned(cleaned))` when the line carries an inline
    ///   `<!-- … -->` region that was stripped before scanning.
    fn process<'a>(&mut self, line: &'a str) -> Option<Cow<'a, str>> {
        let trimmed_start = line.trim_start();
        if self.in_fence {
            if let Some(marker) = self.fence_marker.as_deref()
                && trimmed_start.starts_with(marker)
                && trimmed_start.trim_end().eq(marker)
            {
                self.in_fence = false;
                self.fence_marker = None;
            }
            return None;
        }
        if self.in_comment {
            if let Some(idx) = line.find("-->") {
                self.in_comment = false;
                let after = &line[idx + 3..];
                if let Some(marker) = detect_fence_open(after.trim_start()) {
                    self.in_fence = true;
                    self.fence_marker = Some(marker);
                }
            }
            return None;
        }
        if let Some(marker) = detect_fence_open(trimmed_start) {
            self.in_fence = true;
            self.fence_marker = Some(marker);
            return None;
        }

        // Strip inline `<!-- … -->` regions; only an open without a
        // matching close on the same line flips us into multi-line
        // comment state.
        if !line.contains("<!--") {
            return Some(Cow::Borrowed(line));
        }
        let mut buf = String::with_capacity(line.len());
        let mut rest = line;
        loop {
            let Some(open) = rest.find("<!--") else {
                buf.push_str(rest);
                break;
            };
            buf.push_str(&rest[..open]);
            let after_open = &rest[open + 4..];
            if let Some(close_rel) = after_open.find("-->") {
                rest = &after_open[close_rel + 3..];
            } else {
                self.in_comment = true;
                break;
            }
        }
        Some(Cow::Owned(buf))
    }
}

/// Detect a fence open marker (` ``` ` or `~~~`, optionally followed
/// by an info string) at the start of `line`. Returns the marker
/// string (`"```"` or `"~~~"`) so the matching close can be detected.
fn detect_fence_open(line: &str) -> Option<String> {
    for marker in ["```", "~~~"] {
        if line.starts_with(marker) {
            return Some(marker.to_owned());
        }
    }
    None
}

/// Parse an ATX heading `#{1,6} title`. Trailing `#`s and surrounding
/// whitespace are stripped per `CommonMark` §"ATX headings".
fn parse_heading(line: &str) -> Option<(u8, String)> {
    let mut chars = line.chars();
    let mut hashes: u8 = 0;
    for ch in chars.by_ref() {
        if ch == '#' {
            hashes += 1;
            if hashes > 6 {
                return None;
            }
        } else if ch == ' ' || ch == '\t' {
            break;
        } else {
            return None;
        }
    }
    if hashes == 0 {
        return None;
    }
    let remainder: String = chars.collect();
    let mut title = remainder.trim().to_owned();
    while title.ends_with('#') {
        title.pop();
    }
    let title = title.trim().to_owned();
    if title.is_empty() { None } else { Some((hashes, title)) }
}

/// Scan a single line for `[label](target)` markdown links and
/// `![alt](src)` image embeds. The scanner is intentionally simple:
/// it ignores backslash escapes and reference-style links. Image
/// embeds are recorded with [`MarkdownLink::image`] set so resolution
/// hints can opt in to them (e.g. the missing-diagram-asset check)
/// while plain-link consumers keep their existing flat-link view.
/// Inline code spans (single, double, or triple backticks) are
/// skipped so a literal `` `[label](target)` `` snippet in prose does
/// not surface as a link fact.
fn scan_line_for_links(line: &str, from_path: &str, line_no: u32, out: &mut Vec<MarkdownLink>) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            let run = backtick_run(bytes, i);
            if let Some(close) = find_backtick_run_close(bytes, i + run, run) {
                i = close + run;
                continue;
            }
            break;
        }
        if bytes[i] != b'[' {
            i += 1;
            continue;
        }
        // An `![alt](src)` image embed shares the `[…](…)` shape and
        // is recorded as a link fact flagged `image: true`.
        let image = i > 0 && bytes[i - 1] == b'!';
        let Some(close_bracket) = find_unescaped(bytes, i + 1, b']') else {
            break;
        };
        if close_bracket + 1 >= bytes.len() || bytes[close_bracket + 1] != b'(' {
            i = close_bracket + 1;
            continue;
        }
        let Some(close_paren) = find_unescaped(bytes, close_bracket + 2, b')') else {
            break;
        };
        let target = &line[close_bracket + 2..close_paren];
        let target = target.trim();
        if !target.is_empty() {
            out.push(MarkdownLink {
                from_path: from_path.to_owned(),
                to_raw: target.to_owned(),
                line: line_no,
                resolves: None,
                image,
            });
        }
        i = close_paren + 1;
    }
}

fn backtick_run(bytes: &[u8], start: usize) -> usize {
    let mut n = 0;
    while start + n < bytes.len() && bytes[start + n] == b'`' {
        n += 1;
    }
    n
}

fn find_backtick_run_close(bytes: &[u8], start: usize, run: usize) -> Option<usize> {
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }
        let here = backtick_run(bytes, i);
        if here == run {
            return Some(i);
        }
        i += here;
    }
    None
}

fn find_unescaped(bytes: &[u8], start: usize, needle: u8) -> Option<usize> {
    let mut i = start;
    while i < bytes.len() {
        if bytes[i] == b'\\' {
            i += 2;
            continue;
        }
        if bytes[i] == needle {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Extract closed fenced-code blocks from a markdown file.
#[must_use]
pub fn extract_fenced_blocks(file: &DiscoveredFile) -> Vec<FencedBlock> {
    if file.language.as_deref() != Some("markdown") {
        return Vec::new();
    }
    let text = file.text();
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out = Vec::new();
    let mut in_block = false;
    let mut open_marker = String::new();
    let mut lang = String::new();
    let mut body_start_line = 0_u32;
    let mut body_lines: Vec<String> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        let line_no = u32::try_from(idx + 1).unwrap_or(u32::MAX);
        if !in_block {
            if let Some(caps) = FENCE_OPEN_RE.captures(line) {
                in_block = true;
                open_marker = caps.get(1).map_or("```", |m| m.as_str()).to_string();
                lang = caps.get(2).map_or("", |m| m.as_str()).to_string();
                body_start_line = line_no.saturating_add(1);
                body_lines.clear();
            }
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with(&open_marker) && trimmed.trim_end() == open_marker {
            out.push(FencedBlock {
                path: file.relative.clone(),
                line_start: body_start_line,
                line_end: line_no,
                lang: lang.clone(),
                body: body_lines.join("\n"),
            });
            in_block = false;
            body_lines.clear();
            continue;
        }
        body_lines.push((*line).to_string());
    }
    out
}

#[cfg(test)]
mod tests;

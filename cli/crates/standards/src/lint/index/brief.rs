//! `adapters/**/briefs/**/*.md` extractor per the standards-layer
//! contract §"Module additions".
//!
//! Emits one [`Brief`] fact per markdown file whose project-relative
//! path is either a parent orchestrator brief
//! (`adapters/{sources,targets}/<adapter>/briefs/<op>.md`) or a phase
//! sub-brief (`adapters/{sources,targets}/<adapter>/briefs/{build,extract}/**/*.md`).
//! The two carry a [`BriefScope`] discriminant so size-budget rules can
//! narrow on parent vs phase without re-deriving the path shape.
//! The `sections` array captures `##` heading titles in document
//! order, sharing the fence and HTML-comment state machine from the
//! [`super::markdown`] extractor so fenced "## hidden" headings are
//! not mistaken for body sections. `body_line_count` is the count of
//! non-empty body lines after a leading frontmatter block is stripped.

use super::files::DiscoveredFile;
use super::frontmatter;
use super::markdown::extract_sections;
use crate::lint::{AdapterAxis, Brief, BriefScope};

/// Phase directories whose sub-briefs carry the phase size budget.
const PHASE_DIRS: &[&str] = &["build", "extract"];

/// Extract a [`Brief`] fact from a discovered file.
///
/// Returns `None` when the file does not live under a recognised brief
/// shape (`briefs/<op>.md` parent or `briefs/{build,extract}/**/*.md`
/// phase) or when the file's inferred language is not markdown.
#[must_use]
pub fn extract(file: &DiscoveredFile) -> Option<Brief> {
    if file.language.as_deref() != Some("markdown") {
        return None;
    }
    let (axis, adapter, operation, scope) = parse_brief_path(&file.relative)?;
    let sections: Vec<String> = extract_sections(file)
        .into_iter()
        .filter(|section| section.level == 2)
        .map(|section| section.title)
        .collect();
    let body_line_count = count_body_lines(file);
    Some(Brief {
        path: file.relative.clone(),
        axis,
        adapter: adapter.to_owned(),
        operation: operation.to_owned(),
        scope,
        sections,
        body_line_count,
    })
}

/// Classify a brief path into `(axis, adapter, operation, scope)`.
///
/// A parent brief is `briefs/<op>.md` (a single segment, no nesting);
/// a phase sub-brief is `briefs/{build,extract}/**/*.md` and its
/// `operation` is the phase directory segment.
fn parse_brief_path(relative: &str) -> Option<(AdapterAxis, &str, &str, BriefScope)> {
    let (axis, adapter, tail) = super::path_util::parse_adapter_prefix(relative)?;
    let inner = tail.strip_prefix("briefs/")?;
    let stem = inner.strip_suffix(".md")?;
    match inner.split_once('/') {
        None => {
            if stem.is_empty() {
                return None;
            }
            Some((axis, adapter, stem, BriefScope::Parent))
        }
        Some((phase, _rest)) => {
            if !PHASE_DIRS.contains(&phase) {
                return None;
            }
            Some((axis, adapter, phase, BriefScope::Phase))
        }
    }
}

fn count_body_lines(file: &DiscoveredFile) -> u32 {
    let text = file.text();
    let body = frontmatter::split(&text).map_or(text.as_str(), |(_, body)| body);
    let count = body.lines().filter(|line| !line.trim().is_empty()).count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;

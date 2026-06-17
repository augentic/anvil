//! `plugins/**/SKILL.md` extractor per the standards-layer contract
//! §"Module additions".
//!
//! Emits one [`Skill`] fact per file whose project-relative path
//! matches `plugins/<plugin>/skills/<skill>/SKILL.md`. The plugin
//! slug is the directory immediately under `plugins/`; non-matching
//! paths are skipped silently. The frontmatter is parsed by the
//! shared [`super::frontmatter`] extractor; this extractor only adds
//! the structural facts (name, plugin slug, body line count, and the
//! back-reference into the frontmatter table). Files missing a `name:`
//! frontmatter field collapse to a per-file skip.

use serde_json::Value;

use super::files::DiscoveredFile;
use super::frontmatter;
use crate::lint::Skill;

/// Extract a [`Skill`] fact from a discovered file.
///
/// Returns `None` when the file does not live under
/// `plugins/<plugin>/skills/.../SKILL.md`, when the frontmatter is
/// missing or unparseable, or when the frontmatter lacks a non-empty
/// `name:` field.
#[must_use]
pub fn extract(file: &DiscoveredFile) -> Option<Skill> {
    let (plugin, _skill_slug) = parse_skill_path(&file.relative)?;
    let frontmatter = frontmatter::extract(file)?;
    let name = frontmatter.fields.get("name").and_then(Value::as_str)?.trim();
    if name.is_empty() {
        return None;
    }
    Some(Skill {
        name: name.to_owned(),
        path: file.relative.clone(),
        plugin: plugin.to_owned(),
        frontmatter_ref: format!("{}#frontmatter", file.relative),
        body_line_count: Some(body_line_count(file)),
    })
}

/// Split `plugins/<plugin>/skills/<skill>/SKILL.md` into the
/// `(plugin, skill)` slug pair. Returns `None` for any other shape.
fn parse_skill_path(relative: &str) -> Option<(&str, &str)> {
    let rest = relative.strip_prefix("plugins/")?;
    let (plugin, rest) = rest.split_once('/')?;
    if plugin.is_empty() {
        return None;
    }
    let rest = rest.strip_prefix("skills/")?;
    let (skill, tail) = rest.split_once('/')?;
    if skill.is_empty() || tail != "SKILL.md" {
        return None;
    }
    Some((plugin, skill))
}

/// Count non-frontmatter body lines. Strips a leading
/// `---\n…\n---\n` block before counting.
fn body_line_count(file: &DiscoveredFile) -> u32 {
    let text = file.text();
    let body = frontmatter::split(&text).map_or(text.as_str(), |(_, body)| body);
    u32::try_from(body.lines().count()).unwrap_or(u32::MAX)
}

#[cfg(test)]
mod tests;

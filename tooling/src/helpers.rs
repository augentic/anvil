use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;
use walkdir::WalkDir;

use crate::error::ToolingError;

/// Extract YAML frontmatter from a Markdown file.
///
/// Mirrors `skillFrontmatter()` in `scripts/checks/_shared.ts`.
pub fn skill_frontmatter(content: &str) -> Option<BTreeMap<String, JsonValue>> {
    let body = frontmatter_block(content)?;
    serde_saphyr::from_str(body).ok()
}

/// Return body lines after the closing frontmatter delimiter.
///
/// Mirrors `skillBodyLines()` in `scripts/checks/_shared.ts`.
pub fn skill_body_lines(content: &str) -> Option<Vec<String>> {
    let fm_match = frontmatter_block(content)?;
    let start = content.find(fm_match)? + fm_match.len();
    let mut lines: Vec<String> = content[start..].split('\n').map(str::to_string).collect();
    if lines.first().is_some_and(|line| line.is_empty()) {
        lines.remove(0);
    }
    if lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    Some(lines)
}

/// Strip HTML comments from Markdown before link or prose scans.
pub fn strip_html_comments(content: &str) -> String {
    let mut stripped = String::new();
    let mut cursor = 0;

    while cursor < content.len() {
        let Some(start) = content[cursor..].find("<!--") else {
            stripped.push_str(&content[cursor..]);
            break;
        };
        let start = cursor + start;
        stripped.push_str(&content[cursor..start]);
        let Some(end_rel) = content[start..].find("-->") else {
            break;
        };
        cursor = start + end_rel + "-->".len();
    }

    stripped
}

/// Walk every `SKILL.md` under `plugins/`, skipping symlinked paths.
pub fn walk_skill_files(framework_root: &Path) -> Result<Vec<PathBuf>, ToolingError> {
    walk_matching_files(framework_root, &framework_root.join("plugins"), "SKILL.md")
}

/// Walk every `.md` file under `root`, skipping symlinked paths.
pub fn walk_markdown_files(framework_root: &Path, root: &Path) -> Result<Vec<PathBuf>, ToolingError> {
    walk_matching_files(framework_root, root, ".md")
}

/// Resolve a markdown link target relative to the containing file.
///
/// Mirrors `resolveMarkdownAsset()` in `scripts/checks/docs_quality.ts`.
pub fn resolve_markdown_asset(md_path: &Path, target: &str) -> PathBuf {
    let mut resolved = md_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));

    for part in target.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                resolved.pop();
            }
            _ => resolved.push(part),
        }
    }

    resolved
}

/// Display `path` relative to `framework_root` with forward slashes.
pub fn relative_display(framework_root: &Path, path: &Path) -> String {
    path.strip_prefix(framework_root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

/// Walk files ending with `suffix` under `root`, skipping symlinked paths.
pub fn walk_matching_files(
    framework_root: &Path,
    root: &Path,
    suffix: &str,
) -> Result<Vec<PathBuf>, ToolingError> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in WalkDir::new(root).follow_links(false).into_iter() {
        let entry = entry.map_err(|source| {
            ToolingError::Infrastructure(format!("walk {}: {source}", root.display()))
        })?;
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.into_path();
        if !path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(suffix))
        {
            continue;
        }
        if under_symlink(framework_root, &path)? {
            continue;
        }
        out.push(path);
    }
    out.sort();
    Ok(out)
}

/// True when any ancestor of `path` (relative to `root`) is a symlink.
pub fn under_symlink(root: &Path, path: &Path) -> Result<bool, ToolingError> {
    let rel = path.strip_prefix(root).unwrap_or(path);
    let mut current = root.to_path_buf();
    for part in rel.components().take(rel.components().count().saturating_sub(1)) {
        current.push(part.as_os_str());
        if fs::symlink_metadata(&current)
            .map_err(ToolingError::from)?
            .file_type()
            .is_symlink()
        {
            return Ok(true);
        }
    }
    Ok(fs::symlink_metadata(path)
        .map_err(ToolingError::from)?
        .file_type()
        .is_symlink())
}

fn frontmatter_block(content: &str) -> Option<&str> {
    let rest = content.strip_prefix("---\n")?;
    let end = rest.find("\n---")?;
    Some(&rest[..end])
}

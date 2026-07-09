//! Shared plumbing for the framework-quality predicates: repo walk,
//! frontmatter split, and the fence-aware markdown link scanner.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{Map as JsonMap, Value as JsonValue};

/// One predicate hit: the check id plus a human-readable message that
/// names the offending path (and line where known).
pub struct Finding {
    pub check: &'static str,
    pub message: String,
}

impl Finding {
    pub const fn new(check: &'static str, message: String) -> Self {
        Self { check, message }
    }
}

/// Display `path` relative to `root` with forward slashes.
pub fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

/// Recursive file collector that never follows or records symlinks.
pub fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            walk_files(&path, out);
        } else if file_type.is_file() {
            out.push(path);
        }
    }
}

/// Sorted `.md` files under `dir` (symlinks skipped).
pub fn walk_markdown(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_files(dir, &mut files);
    let mut out: Vec<PathBuf> = files
        .into_iter()
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect();
    out.sort();
    out
}

/// Split `content` at its leading `---` frontmatter block, returning
/// `(yaml_block, body)`. `None` when there is no well-formed block.
pub fn frontmatter_split(content: &str) -> Option<(&str, &str)> {
    let rest = content.strip_prefix("---\n").or_else(|| content.strip_prefix("---\r\n"))?;
    let mut search_from = 0;
    while let Some(pos_rel) = rest[search_from..].find("\n---") {
        let pos = search_from + pos_rel;
        let after = pos + "\n---".len();
        let tail = &rest[after..];
        if tail.is_empty() {
            return Some((&rest[..pos], ""));
        }
        if let Some(body) = tail.strip_prefix('\n').or_else(|| tail.strip_prefix("\r\n")) {
            return Some((&rest[..pos], body));
        }
        search_from = after;
    }
    None
}

/// Parse the frontmatter block into a JSON object map. A missing
/// block returns `None`; a block whose YAML fails to parse (or parses
/// to a non-object) returns an empty map so schema checks still flag
/// the opted-in file.
pub fn parse_frontmatter(content: &str) -> Option<JsonMap<String, JsonValue>> {
    let (block, _) = frontmatter_split(content)?;
    match serde_saphyr::from_str::<JsonValue>(block) {
        Ok(JsonValue::Object(map)) => Some(map),
        Ok(_) | Err(_) => Some(JsonMap::new()),
    }
}

/// One `[label](target)` link extracted from a markdown file.
pub struct MarkdownLink {
    /// Raw link target as written.
    pub target: String,
    /// 1-indexed source line.
    pub line: usize,
    /// `true` for `![alt](src)` image embeds.
    pub image: bool,
}

/// Extract `[label](target)` links from markdown text, skipping fenced
/// code blocks, HTML comments, and inline code spans.
pub fn extract_links(text: &str) -> Vec<MarkdownLink> {
    let mut state = ScanState::default();
    let mut links = Vec::new();
    for (idx, line) in text.split('\n').enumerate() {
        let Some(scanned) = state.process(line) else {
            continue;
        };
        scan_line_for_links(scanned.as_ref(), idx + 1, &mut links);
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

fn detect_fence_open(line: &str) -> Option<String> {
    for marker in ["```", "~~~"] {
        if line.starts_with(marker) {
            return Some(marker.to_owned());
        }
    }
    None
}

fn scan_line_for_links(line: &str, line_no: usize, out: &mut Vec<MarkdownLink>) {
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
        let target = line[close_bracket + 2..close_paren].trim();
        if !target.is_empty() {
            out.push(MarkdownLink {
                target: target.to_owned(),
                line: line_no,
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

/// One closed fenced-code block extracted from markdown text.
pub struct FencedBlock {
    /// Info string after the opening fence (e.g. `text`, `rust`).
    pub lang: String,
    /// 1-indexed line of the first body line.
    pub line_start: usize,
    /// Fence body (without the delimiters).
    pub body: String,
}

/// Extract closed fenced-code blocks.
pub fn extract_fenced_blocks(text: &str) -> Vec<FencedBlock> {
    let mut out = Vec::new();
    let mut in_block = false;
    let mut open_marker = String::new();
    let mut lang = String::new();
    let mut body_start_line = 0_usize;
    let mut body_lines: Vec<&str> = Vec::new();

    for (idx, line) in text.split('\n').enumerate() {
        if !in_block {
            let trimmed = line.trim_start();
            if let Some(marker) = fence_open(trimmed) {
                in_block = true;
                trimmed[marker.len()..]
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .clone_into(&mut lang);
                open_marker = marker;
                body_start_line = idx + 2;
                body_lines.clear();
            }
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with(&open_marker) && trimmed.trim_end() == open_marker {
            out.push(FencedBlock {
                lang: lang.clone(),
                line_start: body_start_line,
                body: body_lines.join("\n"),
            });
            in_block = false;
            body_lines.clear();
            continue;
        }
        body_lines.push(line);
    }
    out
}

fn fence_open(trimmed: &str) -> Option<String> {
    for marker in ["```", "~~~"] {
        if trimmed.starts_with(marker) {
            let mut run = marker.len();
            let bytes = trimmed.as_bytes();
            let delimiter = bytes[0];
            while run < bytes.len() && bytes[run] == delimiter {
                run += 1;
            }
            return Some(trimmed[..run].to_owned());
        }
    }
    None
}

/// Resolve a markdown link `target` written in the file at
/// project-relative `from_rel`. `None` means the resolver did not
/// attempt the target (URL scheme, anchor-only, or empty); otherwise
/// whether the joined path exists on disk under `root`.
pub fn resolve_link(root: &Path, from_rel: &str, target: &str) -> Option<bool> {
    if is_url_scheme(target) {
        return None;
    }
    let path_part = target.split(['#', '?']).next().unwrap_or(target);
    if path_part.is_empty() {
        return None;
    }
    let from = Path::new(from_rel);
    let base = from.parent().unwrap_or_else(|| Path::new(""));
    let joined = base.join(path_part);
    let normalised = normalise_relative(&joined);
    Some(root.join(normalised).exists())
}

fn is_url_scheme(target: &str) -> bool {
    let Some(colon) = target.find("://") else {
        return false;
    };
    let scheme = &target[..colon];
    !scheme.is_empty()
        && scheme.chars().all(|c| {
            c.is_ascii_lowercase() || c.is_ascii_digit() || c == '+' || c == '-' || c == '.'
        })
}

/// Collapse `./` segments and resolve `..` against earlier segments
/// without touching the filesystem.
fn normalise_relative(path: &Path) -> PathBuf {
    let mut out: Vec<std::ffi::OsString> = Vec::new();
    for component in path.components() {
        use std::path::Component;
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::Normal(s) => out.push(s.to_os_string()),
            Component::CurDir | Component::Prefix(_) | Component::RootDir => {}
        }
    }
    let mut buf = PathBuf::new();
    for segment in out {
        buf.push(segment);
    }
    buf
}

/// Discover `plugins/<plugin>/skills/<skill>/SKILL.md` files, keyed as
/// `(plugin, skill, project-relative path)` in stable path order.
pub fn discover_skills(root: &Path) -> Vec<(String, String, String)> {
    let plugins_dir = root.join("plugins");
    let mut files = Vec::new();
    walk_files(&plugins_dir, &mut files);
    let mut out = Vec::new();
    for path in files {
        if path.file_name().and_then(|n| n.to_str()) != Some("SKILL.md") {
            continue;
        }
        let relative = rel(root, &path);
        let Some(rest) = relative.strip_prefix("plugins/") else {
            continue;
        };
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() == 4 && parts[1] == "skills" && parts[3] == "SKILL.md" {
            out.push((parts[0].to_owned(), parts[2].to_owned(), relative));
        }
    }
    out.sort_by(|a, b| a.2.cmp(&b.2));
    out
}

/// Build the `plugin -> {skill}` registry from the on-disk plugin
/// tree, used by the skill-directive check.
pub fn skill_registry(root: &Path) -> BTreeMap<String, std::collections::BTreeSet<String>> {
    let mut registry: BTreeMap<String, std::collections::BTreeSet<String>> = BTreeMap::new();
    for (plugin, skill, _) in discover_skills(root) {
        registry.entry(plugin).or_default().insert(skill);
    }
    registry
}

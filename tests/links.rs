//! Relative markdown link integrity under `plugins/` and `docs/`.
//!
//! Embedded judgment prose under `crates/*/prompts/` is out of scope:
//! `crates/prose` link-checks it at embed time and fails the build.

use std::borrow::Cow;
use std::fs;
use std::path::{Path, PathBuf};

/// Docs pages that intentionally cite illustrative asset paths.
const DIAGRAM_EXCLUDED: &[&str] =
    &["docs/assets/diagrams/_STYLE.md", "docs/standards/doc-authoring.md"];

const LINK_SCOPE_PREFIXES: &[&str] = &["plugins/", "docs/"];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tests/ sits under the repo root")
        .to_path_buf()
}

fn findings(root: &Path) -> Vec<String> {
    let mut out = Vec::new();
    for path in scoped_markdown(root, LINK_SCOPE_PREFIXES) {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let relative = rel(root, &path);
        for link in extract_links(&content) {
            if resolve_link(root, &relative, &link.target) != Some(false) {
                continue;
            }
            if link.image {
                let path_part = link.target.split(['#', '?']).next().unwrap_or(&link.target);
                let is_svg =
                    Path::new(path_part).extension().is_some_and(|e| e.eq_ignore_ascii_case("svg"));
                if is_svg
                    && relative.starts_with("docs/")
                    && !relative.starts_with("docs/book/")
                    && !DIAGRAM_EXCLUDED.contains(&relative.as_str())
                {
                    out.push(format!(
                        "{relative}:{} — diagram embed '{}' does not resolve",
                        link.line, link.target
                    ));
                }
                continue;
            }
            out.push(format!(
                "{relative}:{} — link target '{}' does not resolve",
                link.line, link.target
            ));
        }
    }
    out
}

fn scoped_markdown(root: &Path, prefixes: &[&str]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for prefix in prefixes {
        let dir = root.join(prefix.trim_end_matches('/'));
        if dir.is_dir() {
            out.extend(walk_markdown(&dir));
        }
    }
    out.sort();
    out.dedup();
    out
}

fn walk_markdown(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    walk_files(dir, &mut files);
    let mut out: Vec<PathBuf> = files
        .into_iter()
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect();
    out.sort();
    out
}

fn walk_files(dir: &Path, out: &mut Vec<PathBuf>) {
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

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).unwrap_or(path).to_string_lossy().replace('\\', "/")
}

struct MarkdownLink {
    target: String,
    line: usize,
    image: bool,
}

fn extract_links(text: &str) -> Vec<MarkdownLink> {
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

fn resolve_link(root: &Path, from_rel: &str, target: &str) -> Option<bool> {
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
    Some(root.join(normalise_relative(&joined)).exists())
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

fn write(root: &Path, relative: &str, body: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(path, body).expect("write");
}

#[test]
fn repo_links_resolve() {
    let findings = findings(&repo_root());
    assert!(findings.is_empty(), "unresolved links:\n{findings:#?}");
}

#[test]
fn bad_fixtures() {
    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "docs/guide.md", "See [missing](missing.md).\n");
    assert!(findings(dir.path()).iter().any(|f| f.contains("does not resolve")));

    let dir = tempfile::tempdir().expect("tempdir");
    write(dir.path(), "docs/page.md", "![diagram](../assets/gone.svg)\n");
    let hits = findings(dir.path());
    assert!(hits.iter().any(|f| f.contains("diagram embed")));
    assert!(!hits.iter().any(|f| f.contains("link target")));

    let dir = tempfile::tempdir().expect("tempdir");
    write(
        dir.path(),
        "docs/code.md",
        "```md\n[missing](gone.md)\n```\n\nAnd `[missing](gone.md)` inline.\n",
    );
    assert!(findings(dir.path()).is_empty());
}

//! Mechanical identifier-length gate over every workspace `.rs` tree
//! (docs/standards/coding-standards.md § Naming, docs/standards/testing.md
//! § Test naming): declared item / field / variant names ≤ 25 chars.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// Maximum Unicode scalar length of a declared Rust identifier.
const IDENT_CAP: usize = 25;

const ITEM_KINDS: &[&str] =
    &["fn", "struct", "enum", "union", "trait", "type", "const", "static", "mod"];

#[test]
fn caps() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect(&root.join("src"), &mut files);
    collect(&root.join("tests"), &mut files);
    collect(&root.join("examples"), &mut files);
    let crates = fs::read_dir(root.join("crates")).expect("crates directory");
    for entry in crates {
        let crate_dir = entry.expect("crate directory entry").path();
        collect(&crate_dir.join("src"), &mut files);
        collect(&crate_dir.join("tests"), &mut files);
        collect(&crate_dir.join("examples"), &mut files);
    }
    files.sort();

    let mut violations = String::new();
    for file in &files {
        let text = fs::read_to_string(file).expect("read source file");
        let path = file.strip_prefix(root).expect("workspace-relative path");
        for finding in scan(&text) {
            writeln!(
                violations,
                "  {}:{} `{}` is {} chars (cap {IDENT_CAP})",
                path.display(),
                finding.line,
                finding.name,
                finding.name.chars().count()
            )
            .expect("infallible write to String");
        }
    }
    assert!(
        violations.is_empty(),
        "identifier-length cap violations (docs/standards/coding-standards.md § Naming):\n{violations}"
    );
}

/// Recursively gather `.rs` files under `dir` (absent dirs are fine).
fn collect(dir: &Path, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries {
        let path = entry.expect("source directory entry").path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|found| found == "rs") {
            files.push(path);
        }
    }
}

struct Finding {
    line: usize,
    name: String,
}

fn scan(text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut in_block_comment = false;

    for (index, raw) in text.lines().enumerate() {
        let number = index + 1;
        let line = strip_line_comment(raw, &mut in_block_comment);
        let trimmed = line.trim_start();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(name) = item_name(trimmed) {
            push(&mut findings, number, name);
        }
        if let Some(name) = field_name(trimmed) {
            push(&mut findings, number, name);
        }
        if let Some(name) = variant_name(trimmed) {
            push(&mut findings, number, name);
        }
    }
    findings.sort_by(|a, b| (a.line, &a.name).cmp(&(b.line, &b.name)));
    findings.dedup_by(|a, b| a.line == b.line && a.name == b.name);
    findings
}

fn push(findings: &mut Vec<Finding>, line: usize, name: &str) {
    if name.chars().count() > IDENT_CAP {
        findings.push(Finding {
            line,
            name: name.to_owned(),
        });
    }
}

/// `pub(crate) async fn foo` / `struct Foo` / `const FOO` → name.
fn item_name(trimmed: &str) -> Option<&str> {
    let mut rest = strip_vis(trimmed);
    while let Some((word, after)) = split_word(rest) {
        if matches!(word, "async" | "unsafe" | "const" | "default") {
            // `const` is both a qualifier (`const fn`) and a kind
            // (`const NAME`). Prefer kind when the next token is an ident
            // that is not an item kind.
            if word == "const" {
                let (next, _) = split_word(after.trim_start())?;
                if !ITEM_KINDS.contains(&next) {
                    return raw_ident(after.trim_start());
                }
            }
            rest = after.trim_start();
            continue;
        }
        if ITEM_KINDS.contains(&word) {
            return raw_ident(after.trim_start());
        }
        return None;
    }
    None
}

/// `pub(crate) name:` → field name (skips lifetimes / reserved).
fn field_name(trimmed: &str) -> Option<&str> {
    let rest = strip_vis(trimmed);
    if rest.starts_with('\'') {
        return None;
    }
    let name = raw_ident(rest)?;
    let after = rest.get(name.len()..)?.trim_start();
    (after.starts_with(':') && !after.starts_with("::")).then_some(name)
}

/// `VariantName` / `VariantName {` / `VariantName(` / `VariantName =` at
/// line start — `PascalCase` only, so snake fields are not double-counted.
fn variant_name(trimmed: &str) -> Option<&str> {
    if item_name(trimmed).is_some() {
        return None;
    }
    let name = raw_ident(trimmed)?;
    let mut chars = name.chars();
    let first = chars.next()?;
    if !first.is_uppercase() {
        return None;
    }
    let after = trimmed.get(name.len()..)?.trim_start();
    (after.is_empty()
        || after.starts_with('{')
        || after.starts_with('(')
        || after.starts_with(',')
        || after.starts_with('='))
    .then_some(name)
}

fn strip_vis(trimmed: &str) -> &str {
    trimmed.strip_prefix("pub").map_or(trimmed, |after| {
        let after = after.trim_start();
        after
            .strip_prefix('(')
            .map_or(after, |rest| rest.find(')').map_or(after, |idx| rest[idx + 1..].trim_start()))
    })
}

fn split_word(input: &str) -> Option<(&str, &str)> {
    let end = input.find(|c: char| !c.is_ascii_alphanumeric() && c != '_').unwrap_or(input.len());
    if end == 0 {
        return None;
    }
    Some((&input[..end], &input[end..]))
}

fn raw_ident(input: &str) -> Option<&str> {
    let input = input.strip_prefix("r#").unwrap_or(input);
    let end = input.find(|c: char| !c.is_ascii_alphanumeric() && c != '_').unwrap_or(input.len());
    if end == 0 {
        return None;
    }
    let name = &input[..end];
    let mut chars = name.chars();
    let first = chars.next()?;
    (first.is_ascii_alphabetic() || first == '_').then_some(name)
}

/// Drop `//` line comments and track `/* … */` so comment text does not
/// inflate the gate. Not a full lexer — good enough for declaration heads.
fn strip_line_comment<'a>(line: &'a str, in_block: &mut bool) -> &'a str {
    let bytes = line.as_bytes();
    let mut i = 0;
    let mut out_start = 0;
    while i < bytes.len() {
        if *in_block {
            if bytes[i] == b'*' && i + 1 < bytes.len() && bytes[i + 1] == b'/' {
                *in_block = false;
                i += 2;
                out_start = i;
                continue;
            }
            i += 1;
            continue;
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() {
            match bytes[i + 1] {
                b'/' => return &line[..i],
                b'*' => {
                    *in_block = true;
                    i += 2;
                    continue;
                }
                _ => {}
            }
        }
        i += 1;
    }
    if *in_block { "" } else { &line[out_start..] }
}

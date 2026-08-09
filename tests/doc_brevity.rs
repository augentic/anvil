//! Mechanical doc-density gate over every workspace `src/` tree and
//! WIT contract (docs/standards/coding-standards.md § Comments):
//! module `//!` docs 1–3 prose lines, `///` overviews under 8 before
//! a `#` section, `//` runs ≤ 3.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};

/// Non-blank `//!` lines allowed per module-doc block.
const MODULE_DOC_CAP: usize = 3;
/// Non-blank `///` overview lines allowed before the first `#` section.
const ITEM_DOC_CAP: usize = 8;
/// Consecutive non-blank `//` lines allowed per run.
const LINE_COMMENT_CAP: usize = 3;

#[test]
fn caps() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = vec![root.join("build.rs")];
    collect(&root.join("src"), "rs", &mut files);
    collect(&root.join("wit"), "wit", &mut files);
    let crates = fs::read_dir(root.join("crates")).expect("crates directory");
    for entry in crates {
        let crate_dir = entry.expect("crate directory entry").path();
        collect(&crate_dir.join("src"), "rs", &mut files);
        collect(&crate_dir.join("wit"), "wit", &mut files);
    }
    files.sort();

    let mut violations = String::new();
    for file in &files {
        let text = fs::read_to_string(file).expect("read source file");
        let path = file.strip_prefix(root).expect("workspace-relative path");
        for finding in scan(&text) {
            writeln!(violations, "  {}:{} {}", path.display(), finding.line, finding.message)
                .expect("infallible write to String");
        }
    }
    assert!(
        violations.is_empty(),
        "doc-density cap violations (docs/standards/coding-standards.md § Comments):\n{violations}"
    );
}

/// Recursively gather `.{ext}` files under `dir` (absent dirs are fine).
fn collect(dir: &Path, ext: &str, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries {
        let path = entry.expect("source directory entry").path();
        if path.is_dir() {
            // `wit/deps` symlinks back into crate-owned WIT packages;
            // skip it so shared contracts are not double-counted.
            if path.file_name().is_some_and(|name| name == "deps") {
                continue;
            }
            collect(&path, ext, files);
        } else if path.extension().is_some_and(|found| found == ext) {
            files.push(path);
        }
    }
}

struct Finding {
    line: usize,
    message: String,
}

/// One contiguous comment block under measurement.
struct Block {
    kind: Kind,
    start: usize,
    /// Non-blank, non-fenced prose lines counted against the cap.
    prose: usize,
    in_fence: bool,
    /// An item-doc `# Section` heading was seen; later lines are exempt.
    sectioned: bool,
}

#[derive(PartialEq, Clone, Copy)]
enum Kind {
    Module,
    Item,
    Line,
}

impl Kind {
    fn classify(trimmed: &str) -> Option<(Self, &str)> {
        trimmed
            .strip_prefix("//!")
            .map(|rest| (Self::Module, rest))
            .or_else(|| trimmed.strip_prefix("///").map(|rest| (Self::Item, rest)))
            .or_else(|| trimmed.strip_prefix("//").map(|rest| (Self::Line, rest)))
    }

    const fn cap(self) -> usize {
        match self {
            Self::Module => MODULE_DOC_CAP,
            Self::Item => ITEM_DOC_CAP,
            Self::Line => LINE_COMMENT_CAP,
        }
    }

    const fn label(self) -> &'static str {
        match self {
            Self::Module => "module doc (`//!`)",
            Self::Item => "item doc (`///`) overview",
            Self::Line => "line comment (`//`) run",
        }
    }
}

/// Line-based scan: group consecutive same-kind comment lines into
/// blocks, count prose lines (fenced code and blank separators exempt),
/// and report every block over its kind's cap.
fn scan(text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let mut block: Option<Block> = None;

    for (index, line) in text.lines().enumerate() {
        let number = index + 1;
        let Some((kind, rest)) = Kind::classify(line.trim_start()) else {
            flush(&mut block, &mut findings);
            continue;
        };
        if block.as_ref().is_none_or(|open| open.kind != kind) {
            flush(&mut block, &mut findings);
            block = Some(Block {
                kind,
                start: number,
                prose: 0,
                in_fence: false,
                sectioned: false,
            });
        }
        let open = block.as_mut().expect("block opened above");
        let content = rest.trim();
        if content.starts_with("```") {
            open.in_fence = !open.in_fence;
            continue;
        }
        if kind == Kind::Item && content.starts_with('#') {
            open.sectioned = true;
        }
        if !content.is_empty() && !open.in_fence && !open.sectioned {
            open.prose += 1;
        }
    }
    flush(&mut block, &mut findings);
    findings
}

fn flush(block: &mut Option<Block>, findings: &mut Vec<Finding>) {
    if let Some(open) = block.take()
        && open.prose > open.kind.cap()
    {
        findings.push(Finding {
            line: open.start,
            message: format!(
                "{} runs {} prose lines (cap {})",
                open.kind.label(),
                open.prose,
                open.kind.cap()
            ),
        });
    }
}

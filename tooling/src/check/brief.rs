use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::context::Context;
use crate::error::ToolingError;
use crate::finding::{Check, Finding, Location};
use crate::helpers::under_symlink;

const PARENT_BRIEF_HARD_CAP: usize = 150;
const PHASE_BRIEF_SOFT_CAP: usize = 500;
const PHASE_BRIEF_HARD_CAP: usize = 800;

const RULE_EXCEEDS_SIZE: &str = "brief.exceeds-size-limit";
const RULE_FRONTMATTER_FORBIDDEN: &str = "brief.frontmatter-forbidden";

static PARENT_BRIEF_NAMES: &[&str] = &[
    "shape.md",
    "build.md",
    "merge.md",
    "enumerate.md",
    "extract.md",
];

/// Brief size limits and no-frontmatter discipline for adapter briefs.
pub struct BriefCheck;

impl Check for BriefCheck {
    fn run(&self, ctx: &Context) -> Vec<Finding> {
        run(ctx)
    }
}

/// Run brief size and frontmatter checks against `ctx`.
pub fn run(ctx: &Context) -> Vec<Finding> {
    let root = ctx.framework_root();
    let briefs = match walk_briefs(root) {
        Ok(briefs) => briefs,
        Err(error) => {
            eprintln!("error: brief walk: {error}");
            return Vec::new();
        }
    };

    let mut findings = Vec::new();
    for (path, rel_path) in briefs {
        let parent = is_parent_brief(&rel_path);
        let phase = !parent && is_phase_sub_brief(&rel_path);
        if !parent && !phase {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(source) => {
                findings.push(finding(
                    RULE_EXCEEDS_SIZE,
                    format!("{rel_path}: cannot read brief: {source}"),
                    Some(path),
                ));
                continue;
            }
        };

        findings.extend(check_frontmatter(&rel_path, &content, &path));
        findings.extend(check_size(&rel_path, &content, parent, phase, &path));
    }

    findings
}

fn check_frontmatter(rel_path: &str, content: &str, path: &Path) -> Vec<Finding> {
    if content.starts_with("---\n") || content.starts_with("---\r\n") {
        return vec![finding(
            RULE_FRONTMATTER_FORBIDDEN,
            format!(
                "{rel_path}: brief has YAML frontmatter. Briefs are not skills — \
                 they are resolved by path from adapter.yaml and the loader \
                 never reads brief frontmatter. Strip the leading '---' block \
                 and rely on the body H1 for the brief title. See \
                 docs/standards/skill-authoring.md#brief-authoring."
            ),
            Some(path.to_path_buf()),
        )];
    }
    Vec::new()
}

fn check_size(rel_path: &str, content: &str, parent: bool, phase: bool, path: &Path) -> Vec<Finding> {
    let lines = count_non_blank_lines(content);
    let path = path.to_path_buf();

    if parent && lines > PARENT_BRIEF_HARD_CAP {
        return vec![finding(
            RULE_EXCEEDS_SIZE,
            format!(
                "{rel_path}: parent brief is {lines} non-blank lines, \
                 exceeds hard cap {PARENT_BRIEF_HARD_CAP}. Parent briefs orchestrate; \
                 move operational depth into a phase sub-brief under \
                 {}/<phase>.md or into plugins/<name>/references/.",
                rel_path.trim_end_matches(".md")
            ),
            Some(path),
        )];
    }

    if phase && lines > PHASE_BRIEF_HARD_CAP {
        return vec![finding(
            RULE_EXCEEDS_SIZE,
            format!(
                "{rel_path}: phase sub-brief is {lines} non-blank lines, \
                 exceeds hard cap {PHASE_BRIEF_HARD_CAP}. Split into sub-phases \
                 or move material into plugins/<name>/references/."
            ),
            Some(path),
        )];
    }

    if phase && lines > PHASE_BRIEF_SOFT_CAP {
        eprintln!(
            "WARN: {rel_path}: phase sub-brief is {lines} non-blank lines, \
             above soft cap {PHASE_BRIEF_SOFT_CAP}. Consider moving worked \
             examples and templates into plugins/<name>/references/."
        );
    }

    Vec::new()
}

/// Count non-blank lines, ignoring HTML block and inline comments.
pub fn count_non_blank_lines(content: &str) -> usize {
    let mut count = 0;
    let mut in_block_comment = false;

    for raw in content.split('\n') {
        let line = raw.trim();
        if in_block_comment {
            if line.contains("-->") {
                in_block_comment = false;
            }
            continue;
        }
        if line.is_empty() {
            continue;
        }
        if line.starts_with("<!--") && !line.contains("-->") {
            in_block_comment = true;
            continue;
        }
        if line.starts_with("<!--") && line.contains("-->") {
            continue;
        }
        count += 1;
    }

    count
}

/// True for parent orchestrator briefs at `adapters/<axis>/<adapter>/briefs/{shape,build,merge,enumerate,extract}.md`.
pub fn is_parent_brief(rel_path: &str) -> bool {
    let parts: Vec<&str> = rel_path.split('/').collect();
    if parts.len() != 5 {
        return false;
    }
    if parts[0] != "adapters" {
        return false;
    }
    if parts[1] != "targets" && parts[1] != "sources" {
        return false;
    }
    if parts[3] != "briefs" {
        return false;
    }
    PARENT_BRIEF_NAMES.contains(&parts[4])
}

/// True for phase sub-briefs under `adapters/<axis>/<adapter>/briefs/{build,extract}/**/*.md`.
pub fn is_phase_sub_brief(rel_path: &str) -> bool {
    let parts: Vec<&str> = rel_path.split('/').collect();
    if parts.len() < 6 {
        return false;
    }
    if parts[0] != "adapters" {
        return false;
    }
    if parts[1] != "targets" && parts[1] != "sources" {
        return false;
    }
    if parts[3] != "briefs" {
        return false;
    }
    if parts[4] != "build" && parts[4] != "extract" {
        return false;
    }
    rel_path.ends_with(".md")
}

fn walk_briefs(root: &Path) -> Result<Vec<(PathBuf, String)>, ToolingError> {
    let mut out = Vec::new();

    for axis in ["targets", "sources"] {
        let axis_root = root.join("adapters").join(axis);
        if !axis_root.is_dir() {
            continue;
        }

        for entry in WalkDir::new(&axis_root).follow_links(false).into_iter() {
            let entry = entry.map_err(|source| {
                ToolingError::Infrastructure(format!("walk {}: {source}", axis_root.display()))
            })?;
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.into_path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            if under_symlink(root, &path)? {
                continue;
            }
            let rel_path = path_relative(root, &path);
            out.push((path, rel_path));
        }
    }

    out.sort_by(|a, b| a.1.cmp(&b.1));
    Ok(out)
}

fn path_relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|rel| rel.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.display().to_string())
}

fn finding(rule_id: &'static str, message: String, path: Option<PathBuf>) -> Finding {
    Finding {
        rule_id,
        message,
        location: path.map(|path| Location {
            path,
            line: 1,
            column: None,
        }),
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn count_non_blank_lines_ignores_html_comments() {
        let content = "line one\n\n<!-- block\nstill comment\n-->\nline two\n<!-- inline -->\n";
        assert_eq!(count_non_blank_lines(content), 2);
    }

    #[test]
    fn parent_brief_path_classification() {
        assert!(is_parent_brief("adapters/targets/omnia/briefs/build.md"));
        assert!(!is_parent_brief("adapters/targets/omnia/briefs/build/crate.md"));
    }

    #[test]
    fn phase_sub_brief_path_classification() {
        assert!(is_phase_sub_brief(
            "adapters/targets/omnia/briefs/build/crate.md"
        ));
        assert!(!is_phase_sub_brief("adapters/targets/omnia/briefs/shape.md"));
    }
}

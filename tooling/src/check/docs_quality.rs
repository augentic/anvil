use std::fs;

use crate::context::Context;
use crate::finding::{Check, Finding, Location};
use crate::helpers::{relative_display, resolve_markdown_asset, walk_markdown_files};

const RULE_RFC_CITATION: &str = "docs.rfc-citation-in-docs";
const RULE_MISSING_DIAGRAM: &str = "docs.missing-diagram-asset";
const RULE_TEXT_PIPELINE: &str = "docs.text-pipeline-diagram";

const RFC_ALLOWED_PREFIXES: &[&str] = &[
    "docs/explanation/decision-log.md",
    "docs/explanation/release-notes.md",
    "docs/contributing/",
];

const TEXT_DIAGRAM_ROOTS: &[&str] = &[
    "docs/explanation",
    "docs/orientation",
    "docs/tutorials",
    "docs/how-to",
];

const TEXT_FENCE_ALLOWLIST: &[&str] = &[];

/// Flag RFC citations in user-facing docs outside the decision log and release notes.
pub struct RfcCitationInDocs;

impl Check for RfcCitationInDocs {
    fn id(&self) -> &'static str {
        RULE_RFC_CITATION
    }

    fn run(&self, ctx: &Context) -> Vec<Finding> {
        let root = ctx.framework_root().join("docs");
        if !root.is_dir() {
            return Vec::new();
        }

        let mut findings = Vec::new();
        let files = walk_markdown_files(ctx.framework_root(), &root).unwrap_or_default();

        for path in files {
            let rel = relative_display(ctx.framework_root(), &path);
            if rel.starts_with("docs/assets/") {
                continue;
            }
            if RFC_ALLOWED_PREFIXES
                .iter()
                .any(|prefix| rel.starts_with(prefix))
            {
                continue;
            }

            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(_) => continue,
            };

            for (line_idx, line) in content.lines().enumerate() {
                let stripped = strip_link_targets(line);
                if has_rfc_citation(&stripped) {
                    findings.push(Finding {
                        rule_id: RULE_RFC_CITATION,
                        message: format!(
                            "RFC citation in user-facing docs at {rel}:{} -- {} -- move RFC context to docs/explanation/decision-log.md or strip",
                            line_idx + 1,
                            line.trim()
                        ),
                        location: Some(Location {
                            path: path.clone(),
                            line: line_idx + 1,
                            column: None,
                        }),
                    });
                }
            }
        }

        findings
    }
}

/// Ensure markdown SVG image references resolve to committed assets.
pub struct MissingDiagramAsset;

impl Check for MissingDiagramAsset {
    fn id(&self) -> &'static str {
        RULE_MISSING_DIAGRAM
    }

    fn run(&self, ctx: &Context) -> Vec<Finding> {
        let root = ctx.framework_root().join("docs");
        if !root.is_dir() {
            return Vec::new();
        }

        let mut findings = Vec::new();
        let files = walk_markdown_files(ctx.framework_root(), &root).unwrap_or_default();

        for path in files {
            let rel = relative_display(ctx.framework_root(), &path);
            if rel.starts_with("docs/book/") {
                continue;
            }
            if rel == "docs/assets/diagrams/_STYLE.md"
                || rel == "docs/standards/doc-authoring.md"
            {
                continue;
            }

            let content = match fs::read_to_string(&path) {
                Ok(content) => content,
                Err(_) => continue,
            };

            for target in find_svg_image_refs(&content) {
                if target.starts_with("http://") || target.starts_with("https://") {
                    continue;
                }
                let abs = resolve_markdown_asset(&path, &target);
                if abs.is_file() {
                    continue;
                }
                let resolved = relative_display(ctx.framework_root(), &abs);
                findings.push(Finding {
                    rule_id: RULE_MISSING_DIAGRAM,
                    message: format!(
                        "{rel} references missing SVG {target} (resolved {resolved})"
                    ),
                    location: Some(Location {
                        path: path.clone(),
                        line: 1,
                        column: None,
                    }),
                });
            }
        }

        findings
    }
}

/// Ban ```text flow diagrams under explanation/orientation/tutorials/how-to.
pub struct TextPipelineDiagram;

impl Check for TextPipelineDiagram {
    fn id(&self) -> &'static str {
        RULE_TEXT_PIPELINE
    }

    fn run(&self, ctx: &Context) -> Vec<Finding> {
        let mut findings = Vec::new();

        for rel_root in TEXT_DIAGRAM_ROOTS {
            let root = ctx.framework_root().join(rel_root);
            if !root.is_dir() {
                continue;
            }

            let files = walk_markdown_files(ctx.framework_root(), &root).unwrap_or_default();
            for path in files {
                let rel = relative_display(ctx.framework_root(), &path);
                if TEXT_FENCE_ALLOWLIST.contains(&rel.as_str()) {
                    continue;
                }

                let content = match fs::read_to_string(&path) {
                    Ok(content) => content,
                    Err(_) => continue,
                };

                for block in find_text_fence_blocks(&content) {
                    if has_text_diagram_arrow(&block) {
                        findings.push(Finding {
                            rule_id: RULE_TEXT_PIPELINE,
                            message: format!(
                                "{rel} uses a ```text flow diagram — replace with SVG under docs/assets/diagrams/ (see docs/assets/diagrams/_STYLE.md)"
                            ),
                            location: Some(Location {
                                path: path.clone(),
                                line: 1,
                                column: None,
                            }),
                        });
                    }
                }
            }
        }

        findings
    }
}

fn strip_link_targets(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == ']' && chars.peek() == Some(&'(') {
            chars.next();
            while chars.next().is_some_and(|c| c != ')') {}
            continue;
        }
        out.push(ch);
    }
    out
}

fn has_rfc_citation(text: &str) -> bool {
    for (idx, _) in text.match_indices("RFC") {
        let mut rest = &text[idx + 3..];
        if rest.starts_with('-') || rest.starts_with(' ') {
            rest = &rest[1..];
        }
        if rest
            .chars()
            .next()
            .is_some_and(|ch| ch.is_ascii_digit())
        {
            return true;
        }
    }
    false
}

fn find_svg_image_refs(content: &str) -> Vec<String> {
    let mut refs = Vec::new();
    let mut search = content;
    while let Some(start) = search.find("![") {
        let after = &search[start + 2..];
        let Some(bracket_end) = after.find(']') else {
            search = &search[start + 2..];
            continue;
        };
        let after_bracket = &after[bracket_end + 1..];
        let Some(target) = after_bracket
            .strip_prefix('(')
            .and_then(|inner| inner.split(')').next())
        else {
            search = &search[start + 2..];
            continue;
        };
        if target.ends_with(".svg") {
            refs.push(target.to_string());
        }
        search = &after_bracket[target.len() + 2..];
    }
    refs
}

fn find_text_fence_blocks(content: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut rest = content;
    while let Some(start) = rest.find("```text") {
        let after_open = &rest[start + "```text".len()..];
        let Some(end) = after_open.find("```") else {
            break;
        };
        blocks.push(after_open[..end].to_string());
        rest = &after_open[end + "```".len()..];
    }
    blocks
}

fn has_text_diagram_arrow(block: &str) -> bool {
    block.contains("->") || block.contains('→')
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn strip_link_targets_removes_markdown_urls() {
        assert_eq!(
            strip_link_targets("See [details](rfcs/done/rfc-5.md) for more."),
            "See [details for more."
        );
    }

    #[test]
    fn has_rfc_citation_detects_numbered_refs() {
        assert!(has_rfc_citation("See RFC-5 for details"));
        assert!(has_rfc_citation("See RFC 5 for details"));
        assert!(!has_rfc_citation("See [details](rfcs/rfc-5.md)"));
    }

    #[test]
    fn find_text_fence_blocks_collects_inner_content() {
        let content = "Intro\n\n```text\nA -> B\n```\n\nOutro";
        let blocks = find_text_fence_blocks(content);
        assert_eq!(blocks.len(), 1);
        assert!(has_text_diagram_arrow(&blocks[0]));
    }
}

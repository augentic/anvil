//! Link-integrity predicates: relative markdown links, SVG diagram
//! embeds, skill directives, plugin symlinks, and the
//! docs/-in-deployable-surface ban.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;

use crate::support::{
    Finding, extract_links, normalise_relative, rel, resolve_link, skill_registry, walk_markdown,
};

/// A relative `[label](target)` markdown link does not resolve on disk.
pub const CHECK_LINK_UNRESOLVED: &str = "links.unresolved";
/// A `![alt](….svg)` diagram embed under `docs/` does not resolve.
pub const CHECK_DIAGRAM_ASSET_MISSING: &str = "links.missing-diagram-asset";
/// A `<!-- skill: plugin:skill -->` directive does not resolve.
pub const CHECK_DIRECTIVE_UNRESOLVED: &str = "links.unresolved-directive";
/// A symlink under `plugins/` points at a missing target.
pub const CHECK_BROKEN_SYMLINK: &str = "plugins.broken-symlink";
/// A deployable surface links into the repo's `docs/` tree.
pub const CHECK_DOCS_IN_DEPLOYABLE: &str = "links.docs-in-deployable-surface";
/// A permanent surface links into the disposable `rfcs/` tree.
pub const CHECK_RFCS_LINK: &str = "links.rfcs-in-permanent-surface";

/// Markdown trees whose relative links must resolve on disk. Archival
/// trees (`rfcs/`) are excluded from *inbound* resolution by design:
/// they cite future or deferred work whose targets may not exist yet.
/// *Outbound* links into `rfcs/` from permanent surfaces are banned
/// separately ([`CHECK_RFCS_LINK`]). The embedded judgment-prose
/// corpora under `crates/slice/prompts/` and `crates/change/prompts/`
/// are out of scope: `crates/prose` link-checks them at embed time and
/// fails the build on a dangling reference.
const LINK_SCOPE_PREFIXES: &[&str] = &["plugins/", "docs/", ".cursor/"];

/// Trees walked for skill directives (the framework include set).
const DIRECTIVE_SCOPE_PREFIXES: &[&str] = &["plugins/", "docs/", ".cursor/", "rfcs/", "schemas/"];

/// Docs pages excluded from the SVG-embed check: rendered output and
/// pages that intentionally cite illustrative asset paths.
const DIAGRAM_EXCLUDED: &[&str] =
    &["docs/assets/diagrams/_STYLE.md", "docs/standards/doc-authoring.md"];

/// Run every link predicate rooted at `root`.
pub fn run(root: &Path) -> Vec<Finding> {
    let mut findings = Vec::new();
    check_markdown_links(root, &mut findings);
    check_directives(root, &mut findings);
    check_plugin_symlinks(root, &mut findings);
    check_docs_links_in_deployable(root, &mut findings);
    check_rfcs_links(root, &mut findings);
    findings
}

/// Relative-link resolution over the scoped markdown trees: plain
/// links must resolve (CORE-002/019 lineage) and `.svg` image embeds
/// under `docs/` must resolve (CORE-015 lineage).
fn check_markdown_links(root: &Path, findings: &mut Vec<Finding>) {
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
                    findings.push(Finding::new(
                        CHECK_DIAGRAM_ASSET_MISSING,
                        format!(
                            "{relative}:{} — diagram embed '{}' does not resolve on disk",
                            link.line, link.target
                        ),
                    ));
                }
                continue;
            }
            findings.push(Finding::new(
                CHECK_LINK_UNRESOLVED,
                format!(
                    "{relative}:{} — link target '{}' does not resolve on disk",
                    link.line, link.target
                ),
            ));
        }
    }
}

static INLINE_CODE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"`[^`]+`").expect("inline code pattern"));

static DIRECTIVE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"<!-- skill: ([a-z][a-z0-9-]*):([a-z][a-z0-9-]*) -->").expect("directive pattern")
});
static FENCE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"```[\s\S]*?```").expect("fence pattern"));

/// Every `<!-- skill: plugin:skill -->` directive must resolve against
/// the on-disk skill registry under `plugins/`.
fn check_directives(root: &Path, findings: &mut Vec<Finding>) {
    let registry = skill_registry(root);
    let mut scope: Vec<PathBuf> = scoped_markdown(root, DIRECTIVE_SCOPE_PREFIXES);
    scope.extend(root_markdown(root));
    for path in scope {
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        let relative = rel(root, &path);
        let no_fence = FENCE_RE.replace_all(&content, "");
        let stripped = INLINE_CODE_RE.replace_all(&no_fence, "");
        for cap in DIRECTIVE_RE.captures_iter(&stripped) {
            let plugin = cap.get(1).map_or("", |m| m.as_str());
            let skill = cap.get(2).map_or("", |m| m.as_str());
            match registry.get(plugin) {
                None => findings.push(Finding::new(
                    CHECK_DIRECTIVE_UNRESOLVED,
                    format!("{relative} — skill directive plugin '{plugin}' not found"),
                )),
                Some(skills) if !skills.contains(skill) => findings.push(Finding::new(
                    CHECK_DIRECTIVE_UNRESOLVED,
                    format!("{relative} — skill directive '{plugin}:{skill}' not found"),
                )),
                Some(_) => {}
            }
        }
    }
}

/// Every symlink under `plugins/` must resolve to an existing target.
fn check_plugin_symlinks(root: &Path, findings: &mut Vec<Finding>) {
    let mut symlinks = Vec::new();
    collect_symlinks(&root.join("plugins"), &mut symlinks);
    for path in symlinks {
        if fs::metadata(&path).is_ok() {
            continue;
        }
        let target = fs::read_link(&path)
            .map_or_else(|_| "<unreadable>".to_owned(), |t| t.display().to_string());
        findings.push(Finding::new(
            CHECK_BROKEN_SYMLINK,
            format!("{} — symlink target '{target}' does not resolve", rel(root, &path)),
        ));
    }
}

fn collect_symlinks(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        let path = entry.path();
        if file_type.is_symlink() {
            out.push(path);
        } else if file_type.is_dir() {
            collect_symlinks(&path, out);
        }
    }
}

static DOCS_LINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\]\((docs/|[^)]*\.\./docs/)[^)]*\)").expect("docs link pattern"));

/// Deployable surfaces (plugins) must not link into the contributor
/// `docs/` tree.
fn check_docs_links_in_deployable(root: &Path, findings: &mut Vec<Finding>) {
    for path in scoped_markdown(root, &["plugins/"]) {
        let relative = rel(root, &path);
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for (line_idx, line) in content.lines().enumerate() {
            if DOCS_LINK_RE.is_match(line) {
                findings.push(Finding::new(
                    CHECK_DOCS_IN_DEPLOYABLE,
                    format!("{relative}:{} — deployable surface links into docs/", line_idx + 1),
                ));
            }
        }
    }
}

/// Permanent markdown trees banned from linking into the disposable
/// `rfcs/` design parking lot. The embedded prompt corpora stay in
/// scope: the compile-time link check only proves targets resolve,
/// and an rfcs/ path resolves fine while still being banned.
const RFCS_BAN_PREFIXES: &[&str] =
    &["docs/", "plugins/", "crates/slice/prompts/", "crates/change/prompts/"];

/// Permanent surfaces must not link into `rfcs/` — its contents are
/// disposable working design, deleted once implemented.
fn check_rfcs_links(root: &Path, findings: &mut Vec<Finding>) {
    for path in scoped_markdown(root, RFCS_BAN_PREFIXES) {
        let relative = rel(root, &path);
        let Ok(content) = fs::read_to_string(&path) else {
            continue;
        };
        for link in extract_links(&content) {
            let path_part = link.target.split(['#', '?']).next().unwrap_or(&link.target);
            if path_part.is_empty() || link.target.contains("://") {
                continue;
            }
            let base = Path::new(&relative).parent().unwrap_or_else(|| Path::new(""));
            let joined = normalise_relative(&base.join(path_part));
            if joined.starts_with("rfcs") {
                findings.push(Finding::new(
                    CHECK_RFCS_LINK,
                    format!(
                        "{relative}:{} — permanent surface links into rfcs/ ('{}')",
                        link.line, link.target
                    ),
                ));
            }
        }
    }
}

/// Markdown files under any of the given root-relative prefixes.
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

/// Root-level `*.md` files (e.g. `AGENTS.md`, `README.md`).
fn root_markdown(root: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let mut out: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_file()))
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|e| e.to_str()) == Some("md"))
        .collect();
    out.sort();
    out
}

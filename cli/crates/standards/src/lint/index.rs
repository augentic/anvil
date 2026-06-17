//! Project and framework indexer per the standards-layer contract
//! §"`WorkspaceModel`" and the file scan contract.
//!
//! The project scan is a `.gitignore`-aware file walk,
//! per-file extractors that run in parallel via `rayon`, and a
//! sequential second pass that records symlinks and resolves
//! cross-file edges. The framework profile adds
//! a wider include set, follow-the-link symlink
//! traversal with cycle detection, and dedicated extractors for
//! `plugins/**/SKILL.md`, `adapters/**/adapter.yaml`,
//! `**/agent-teams.md` symlinks, and `adapters/**/briefs/*.md`.
//!
//! Both profiles share the same per-file extractors (`frontmatter`,
//! `markdown`, `ignore_directives`) and the same `WorkspaceModel`
//! assembly invariants (byte-stable enumeration, sorted output
//! collections). The umbrella owns the [`build`] entry point and the
//! closed [`IndexError`] enum the runtime maps to exit codes via
//! `Exit::from(&Error)`.

pub mod adapter;
pub mod adapter_dir;
pub mod brief;
pub mod files;
pub mod framework;
pub mod frontmatter;
pub mod ignore_directives;
pub mod languages;
pub mod markdown;
pub mod path_util;
pub mod scenario;
pub mod skill;
pub mod symlinks;

use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::lint::{
    AdapterManifest, Brief, File, Frontmatter, IgnoreDirective, MarkdownLink, MarkdownSection,
    ScanProfile, Skill, Symlink, WorkspaceModel, WorkspaceModelVersion,
};

/// Closed error set for [`build`].
///
/// Per-file extractor failures (malformed YAML frontmatter,
/// unreadable bytes, etc.) collapse to silent per-file skips in v1;
/// reserved-hint diagnostics reserves the `index.warning` finding
/// for the hint runner. Only conditions the indexer cannot
/// meaningfully recover from surface as `Err`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IndexError {
    /// Reserved for an unsupported profile addition; v1 supports
    /// `project` and `framework`. Carried as part of the public
    /// wire contract so existing exit-code mappings continue to
    /// compile.
    #[error("unsupported scan profile: {0:?}")]
    UnsupportedScanProfile(ScanProfile),
    /// `project_dir` is not an existing directory; the walk cannot
    /// proceed.
    #[error("project directory not found: {0}")]
    ProjectDirMissing(PathBuf),
    /// Always-ignore override compilation failed. Indicates a
    /// programmer error in the static glob list.
    #[error("always-ignore override compilation failed: {0}")]
    OverrideCompile(String),
    /// Filesystem-level abort during the framework walk per §F1 —
    /// e.g. a symlink cycle the walker cannot make progress through.
    #[error("filesystem error during framework scan: {0}")]
    Filesystem(String),
}

/// Build the [`WorkspaceModel`] for `project_dir` under the
/// requested profile.
///
/// Under [`ScanProfile::Project`] the walk roots at `project_dir`
/// (or each `artifact_paths` root when supplied) and the indexer
/// runs the project-scope extractors. Under [`ScanProfile::Framework`]
/// the walk applies the §F1 include set, follows symlinks with
/// cycle detection, and runs the framework extractors
/// (`skill`, `adapter`, `marketplace`, `brief`) in
/// addition to the shared markdown / frontmatter passes.
///
/// `languages`, when non-empty, narrows the discovered file set to
/// extensions whose inferred language token matches one of the
/// supplied tokens; unknown / binary files are kept so that
/// symlinks and asset-adjacent files still appear in the model.
///
/// # Errors
///
/// Returns the matching [`IndexError`] variant; see the per-variant
/// documentation. Per-file extractor failures are swallowed silently.
pub fn build(
    project_dir: &Path, scan_profile: ScanProfile, artifact_paths: &[PathBuf], languages: &[String],
) -> Result<WorkspaceModel, IndexError> {
    match scan_profile {
        ScanProfile::Project => build_project(project_dir, artifact_paths, languages),
        ScanProfile::Framework => build_framework(project_dir, artifact_paths, languages),
    }
}

fn build_project(
    project_dir: &Path, artifact_paths: &[PathBuf], languages: &[String],
) -> Result<WorkspaceModel, IndexError> {
    let discovery = files::discover(project_dir, artifact_paths, languages)?;
    let discovered = discovery.files;
    let symlinks_facts = discovery.symlinks;

    let per_file: Vec<PerFile> = discovered
        .into_par_iter()
        .map(|file| {
            let frontmatter = frontmatter::extract(&file);
            let sections = markdown::extract_sections(&file);
            let links = markdown::extract_links(&file);
            let ignore_directives = ignore_directives::extract(&file);
            let fenced_blocks = markdown::extract_fenced_blocks(&file);
            PerFile {
                file: File {
                    path: file.relative,
                    kind: file.kind,
                    language: file.language,
                    sha256: None,
                },
                frontmatter,
                sections,
                links,
                ignore_directives,
                fenced_blocks,
            }
        })
        .collect();

    let mut files_out: Vec<File> = Vec::with_capacity(per_file.len());
    let mut frontmatter_out: Vec<Frontmatter> = Vec::new();
    let mut sections_out: Vec<MarkdownSection> = Vec::new();
    let mut links_out: Vec<MarkdownLink> = Vec::new();
    let mut ignore_directives_out: Vec<IgnoreDirective> = Vec::new();
    let mut fenced_blocks_out: Vec<crate::lint::FencedBlock> = Vec::new();
    for entry in per_file {
        files_out.push(entry.file);
        if let Some(fm) = entry.frontmatter {
            frontmatter_out.push(fm);
        }
        sections_out.extend(entry.sections);
        links_out.extend(entry.links);
        ignore_directives_out.extend(entry.ignore_directives);
        fenced_blocks_out.extend(entry.fenced_blocks);
    }

    let known_paths: std::collections::HashSet<String> =
        files_out.iter().map(|f| f.path.clone()).collect();
    for link in &mut links_out {
        link.resolves = resolve_link(project_dir, &link.from_path, &link.to_raw, &known_paths);
    }

    files_out.sort_by(|a, b| a.path.cmp(&b.path));
    frontmatter_out.sort_by(|a, b| a.path.cmp(&b.path));
    sort_sections(&mut sections_out);
    sort_links(&mut links_out);
    sort_ignore_directives(&mut ignore_directives_out);
    sort_fenced_blocks(&mut fenced_blocks_out);

    Ok(WorkspaceModel {
        version: WorkspaceModelVersion,
        project_dir: project_dir.to_string_lossy().into_owned(),
        scan_profile: ScanProfile::Project,
        artifact_paths: artifact_paths.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        languages: languages.to_vec(),
        files: files_out,
        frontmatter: frontmatter_out,
        markdown_sections: sections_out,
        markdown_links: links_out,
        symlinks: symlinks_facts,
        skills: Vec::new(),
        adapter_manifests: Vec::new(),
        ignore_directives: ignore_directives_out,
        fenced_blocks: fenced_blocks_out,
        briefs: Vec::new(),
        scenarios: Vec::new(),
        adapter_dirs: Vec::new(),
    })
}

fn build_framework(
    project_dir: &Path, artifact_paths: &[PathBuf], languages: &[String],
) -> Result<WorkspaceModel, IndexError> {
    let discovery = framework::discover(project_dir, artifact_paths, languages)?;
    let discovered = discovery.files;
    let symlinks_facts = discovery.symlinks;

    let per_file: Vec<FrameworkPerFile> = discovered
        .into_par_iter()
        .map(|file| {
            let frontmatter = frontmatter::extract(&file);
            let sections = markdown::extract_sections(&file);
            let links = markdown::extract_links(&file);
            let ignore_directives = ignore_directives::extract(&file);
            let skill = skill::extract(&file);
            let manifest = adapter::extract(&file);
            let brief = brief::extract(&file);
            let fenced_blocks = markdown::extract_fenced_blocks(&file);
            FrameworkPerFile {
                file: File {
                    path: file.relative,
                    kind: file.kind,
                    language: file.language,
                    sha256: None,
                },
                frontmatter,
                sections,
                links,
                ignore_directives,
                skill,
                manifest,
                brief,
                fenced_blocks,
            }
        })
        .collect();

    let mut files_out: Vec<File> = Vec::with_capacity(per_file.len());
    let mut frontmatter_out: Vec<Frontmatter> = Vec::new();
    let mut sections_out: Vec<MarkdownSection> = Vec::new();
    let mut links_out: Vec<MarkdownLink> = Vec::new();
    let mut ignore_directives_out: Vec<IgnoreDirective> = Vec::new();
    let mut skills_out: Vec<Skill> = Vec::new();
    let mut manifests_out: Vec<AdapterManifest> = Vec::new();
    let mut briefs_out: Vec<Brief> = Vec::new();
    let mut fenced_blocks_out: Vec<crate::lint::FencedBlock> = Vec::new();
    for entry in per_file {
        files_out.push(entry.file);
        if let Some(fm) = entry.frontmatter {
            frontmatter_out.push(fm);
        }
        sections_out.extend(entry.sections);
        links_out.extend(entry.links);
        ignore_directives_out.extend(entry.ignore_directives);
        fenced_blocks_out.extend(entry.fenced_blocks);
        if let Some(skill) = entry.skill {
            skills_out.push(skill);
        }
        if let Some(manifest) = entry.manifest {
            manifests_out.push(manifest);
        }
        if let Some(brief) = entry.brief {
            briefs_out.push(brief);
        }
    }

    // The framework walker follows symlinks (`follow_links(true)`) so
    // any file reachable both at its canonical path and through a
    // symlinked ancestor is recorded twice — once via each path. The
    // canonical-path copy resolves relative `[label](target)` links
    // correctly; the symlink-traversed copy joins them against the
    // wrong parent and would surface spurious `reference-resolves`
    // failures (CORE-002 et al.). Drop the symlink-traversed copies
    // so reference resolution matches the retired imperative
    // `links.unresolved` predicate's `follow_links(false)` behaviour.
    drop_symlink_traversed_links(&mut links_out, &symlinks_facts);

    let known_paths: std::collections::HashSet<String> =
        files_out.iter().map(|f| f.path.clone()).collect();
    for link in &mut links_out {
        link.resolves = resolve_link(project_dir, &link.from_path, &link.to_raw, &known_paths);
    }

    files_out.sort_by(|a, b| a.path.cmp(&b.path));
    frontmatter_out.sort_by(|a, b| a.path.cmp(&b.path));
    sort_sections(&mut sections_out);
    sort_links(&mut links_out);
    sort_ignore_directives(&mut ignore_directives_out);
    sort_fenced_blocks(&mut fenced_blocks_out);
    skills_out.sort_by(|a, b| a.path.cmp(&b.path));
    manifests_out.sort_by(|a, b| a.path.cmp(&b.path));
    briefs_out.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(WorkspaceModel {
        version: WorkspaceModelVersion,
        project_dir: project_dir.to_string_lossy().into_owned(),
        scan_profile: ScanProfile::Framework,
        artifact_paths: artifact_paths.iter().map(|p| p.to_string_lossy().into_owned()).collect(),
        languages: languages.to_vec(),
        files: files_out,
        frontmatter: frontmatter_out,
        markdown_sections: sections_out,
        markdown_links: links_out,
        symlinks: symlinks_facts,
        skills: skills_out,
        adapter_manifests: manifests_out,
        ignore_directives: ignore_directives_out,
        fenced_blocks: fenced_blocks_out,
        briefs: briefs_out,
        scenarios: scenario::extract(project_dir),
        adapter_dirs: adapter_dir::extract(project_dir),
    })
}

fn sort_fenced_blocks(blocks: &mut [crate::lint::FencedBlock]) {
    blocks.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.line_start.cmp(&b.line_start)));
}

fn sort_sections(sections: &mut [MarkdownSection]) {
    sections.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.line_start.cmp(&b.line_start)));
}

fn sort_links(links: &mut [MarkdownLink]) {
    links.sort_by(|a, b| {
        a.from_path
            .cmp(&b.from_path)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.to_raw.cmp(&b.to_raw))
    });
}

fn sort_ignore_directives(directives: &mut [IgnoreDirective]) {
    directives.sort_by(|a, b| {
        a.path
            .cmp(&b.path)
            .then_with(|| a.line.cmp(&b.line))
            .then_with(|| a.rule_id.cmp(&b.rule_id))
    });
}

struct PerFile {
    file: File,
    frontmatter: Option<Frontmatter>,
    sections: Vec<MarkdownSection>,
    links: Vec<MarkdownLink>,
    ignore_directives: Vec<IgnoreDirective>,
    fenced_blocks: Vec<crate::lint::FencedBlock>,
}

struct FrameworkPerFile {
    file: File,
    frontmatter: Option<Frontmatter>,
    sections: Vec<MarkdownSection>,
    links: Vec<MarkdownLink>,
    ignore_directives: Vec<IgnoreDirective>,
    fenced_blocks: Vec<crate::lint::FencedBlock>,
    skill: Option<Skill>,
    manifest: Option<AdapterManifest>,
    brief: Option<Brief>,
}

/// Strip markdown-link facts whose `from_path` was reached through
/// a symlinked ancestor recorded in `symlinks_facts`. Pairs with
/// [`build_framework`]'s post-discovery cleanup; see the call-site
/// comment for the rationale.
fn drop_symlink_traversed_links(links: &mut Vec<MarkdownLink>, symlinks_facts: &[Symlink]) {
    if symlinks_facts.is_empty() {
        return;
    }
    links.retain(|link| {
        !symlinks_facts.iter().any(|s| {
            link.from_path == s.path
                || link.from_path.starts_with(s.path.as_str())
                    && link.from_path[s.path.len()..].starts_with('/')
        })
    });
}

/// Resolve a markdown link target. URL-style targets (matching
/// `^[a-z][a-z0-9+\-.]*://`) leave `resolves` unset; relative paths
/// are joined against `from_path`'s parent and checked against both
/// the discovered file set and the filesystem.
fn resolve_link(
    project_dir: &Path, from_path: &str, to_raw: &str,
    known_paths: &std::collections::HashSet<String>,
) -> Option<bool> {
    if is_url_scheme(to_raw) {
        return None;
    }
    let target_without_fragment = to_raw.split(['#', '?']).next().unwrap_or(to_raw);
    if target_without_fragment.is_empty() {
        return None;
    }
    let from = Path::new(from_path);
    let base = from.parent().unwrap_or_else(|| Path::new(""));
    let joined = base.join(target_without_fragment);
    let normalised = normalise_relative(&joined);
    let candidate_rel = normalised.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
    if known_paths.contains(&candidate_rel) {
        return Some(true);
    }
    Some(project_dir.join(&candidate_rel).exists())
}

fn is_url_scheme(target: &str) -> bool {
    let Some(colon) = target.find("://") else {
        return false;
    };
    let scheme = &target[..colon];
    if scheme.is_empty() {
        return false;
    }
    scheme
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '+' || c == '-' || c == '.')
}

/// Collapse `./` segments and resolve `..` against earlier segments
/// without touching the filesystem; project-relative paths use
/// forward slashes per `WorkspaceModel` stability.
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

#[cfg(test)]
mod tests;

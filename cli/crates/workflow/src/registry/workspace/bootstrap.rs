//! Greenfield bootstrap: scaffold a fresh workspace slot when a remote
//! clone fails or the slot has no `.specify/project.yaml`.

use std::path::Path;

use specify_error::Error;

use super::git::{self, git_output_ok, git_porcelain_non_empty};
use crate::adapter::TargetAdapter;
use crate::config::Layout;
use crate::init::adapter_ref_from_value;
use crate::registry::gitignore::ensure_gitignore_entries;

pub(super) fn bootstrap(
    url: &str, dest: &Path, adapter: &str, initiating_project_dir: &Path,
) -> Result<(), Error> {
    std::fs::create_dir_all(dest).map_err(Error::Io)?;

    git::run(dest, &["init"], &format!("git init in {}", dest.display()))?;
    git::run(dest, &["remote", "add", "origin", url], &format!("git remote add origin {url}"))?;

    greenfield_init(dest, adapter, initiating_project_dir, false)?;

    Ok(())
}

pub(super) fn greenfield_init(
    dest: &Path, adapter: &str, initiating_project_dir: &Path, is_rerun: bool,
) -> Result<(), Error> {
    let adapter = resolve_greenfield_adapter(adapter, initiating_project_dir)?;

    scaffold_greenfield(dest, &adapter)?;

    git::run(dest, &["add", "."], &format!("git add in {}", dest.display()))?;

    if !git_porcelain_non_empty(dest) {
        return Ok(());
    }

    let has_commits = git_output_ok(dest, &["log", "--oneline", "-1"]).is_some();
    let commit_args = if is_rerun && has_commits {
        vec!["commit", "--amend", "--no-gpg-sign", "-m", "Initial Specify scaffold"]
    } else {
        vec!["commit", "--no-gpg-sign", "-m", "Initial Specify scaffold"]
    };
    git::run(dest, &commit_args, &format!("git commit in {}", dest.display()))?;

    Ok(())
}

fn scaffold_greenfield(dest: &Path, adapter: &str) -> Result<(), Error> {
    let specify_dir = dest.join(".specify");
    // The memoization cache is out-of-tree (OS cache, populated on
    // demand), so only the in-tree `.specify/` tenants are scaffolded.
    for dir in [
        specify_dir.clone(),
        specify_dir.join("slices"),
        specify_dir.join("specs"),
        specify_dir.join("archive"),
    ] {
        std::fs::create_dir_all(&dir).map_err(Error::Io)?;
    }

    let name = dest
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("greenfield");

    let platforms_line = resolve_default_platforms(adapter, dest);

    let mut project_yaml = format!(
        "name: {name}\nadapter: {adapter}\nspecify_version: \"{}\"\nrules: {{}}\n",
        env!("CARGO_PKG_VERSION")
    );
    if let Some(line) = platforms_line {
        project_yaml.push_str(&line);
    }
    std::fs::write(specify_dir.join("project.yaml"), project_yaml).map_err(Error::Io)?;
    ensure_gitignore_entries(dest)?;

    Ok(())
}

/// Attempt to resolve the target adapter and return a YAML `platforms:`
/// line containing the manifest's default set when the target declares
/// `platforms.required`. Returns `None` on resolve failure (edge case:
/// adapter cache not yet populated) or when the target does not require
/// platforms.
fn resolve_default_platforms(adapter: &str, dest: &Path) -> Option<String> {
    let resolved = TargetAdapter::resolve(&adapter_ref_from_value(adapter), dest).ok()?;
    let cap = resolved.manifest.platforms.as_ref()?;
    if !cap.required || cap.default.is_empty() {
        return None;
    }
    let tokens: Vec<String> = cap.default.iter().map(ToString::to_string).collect();
    Some(format!(
        "platforms:\n{}\n",
        tokens.iter().map(|t| format!("  - {t}")).collect::<Vec<_>>().join("\n")
    ))
}

/// Resolve the adapter identifier to pass into a greenfield slot's
/// `specify init <adapter>`.
///
/// URL-shaped adapters are already self-contained. Bare registry
/// adapter identifiers are local to the initiating repo's cache, so
/// convert them into a file URI the spawned init can copy directly.
fn resolve_greenfield_adapter(
    adapter: &str, initiating_project_dir: &Path,
) -> Result<String, Error> {
    if adapter.contains("://") {
        return Ok(adapter.to_string());
    }
    let cache_base = Layout::new(initiating_project_dir).cache_dir();

    let direct = cache_base.join(adapter);
    if direct.is_dir() {
        return Ok(format!("file://{}", direct.display()));
    }

    let without_ref = adapter.split('@').next().unwrap_or(adapter);
    if let Some(segment) = without_ref.rsplit('/').find(|s| !s.is_empty()) {
        let by_segment = cache_base.join(segment);
        if by_segment.is_dir() {
            return Ok(format!("file://{}", by_segment.display()));
        }
    }

    Err(Error::Diag {
        code: "workspace-adapter-not-cached",
        detail: format!(
            "adapter '{}' not cached in {}; run /spec:init in the initiating repo first",
            adapter,
            cache_base.display()
        ),
    })
}

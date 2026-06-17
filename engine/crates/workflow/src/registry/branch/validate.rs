//! Pre-checkout guards: worktree existence, origin presence, and the
//! tracked/untracked classification of the slot's working tree against
//! the active slice's allow-list.

use std::path::{Path, PathBuf};

use super::infer::{AllowedPath, allowed_paths};
use super::{Diagnostic, Dirty, git_output};
use crate::registry::catalog::RegistryProject;

pub(super) fn require_git_worktree(
    slot_path: &Path, project: &RegistryProject, branch: &str,
) -> Result<(), Diagnostic> {
    if !slot_path.exists() {
        return Err(Diagnostic::new(
            "workspace-slot-missing",
            project,
            Some(branch),
            format!(
                "`{}` does not exist; run `specify workspace sync {}` first",
                slot_path.display(),
                project.name
            ),
        ));
    }
    if !slot_path.join(".git").exists() {
        return Err(Diagnostic::new(
            "workspace-slot-not-git",
            project,
            Some(branch),
            format!("`{}` is not a git worktree", slot_path.display()),
        ));
    }
    Ok(())
}

pub(super) fn require_origin(
    slot_path: &Path, project: &RegistryProject, branch: &str,
) -> Result<String, Diagnostic> {
    git_output(slot_path, ["remote", "get-url", "origin"], project, Some(branch)).map_err(|_err| {
        Diagnostic::new(
            "missing-origin",
            project,
            Some(branch),
            format!(
                "`{}` has no origin remote; branch preparation requires a remote default",
                slot_path.display()
            ),
        )
    })
}

pub(super) fn classify_dirty(
    slot_path: &Path, change_name: &str, source_paths: &[PathBuf], output_paths: &[PathBuf],
) -> Dirty {
    let allowed = allowed_paths(slot_path, change_name, source_paths, output_paths);
    let mut tracked_allowed = Vec::new();
    let mut tracked_blocked = Vec::new();
    let mut untracked = Vec::new();

    for entry in porcelain_entries(slot_path) {
        match entry.kind {
            PorcelainKind::Untracked => untracked.push(entry.path),
            PorcelainKind::Tracked => {
                if allowed.iter().any(|allowed_path| allowed_path.matches(&entry.path)) {
                    tracked_allowed.push(entry.path);
                } else {
                    tracked_blocked.push(entry.path);
                }
            }
        }
    }

    tracked_allowed.sort();
    tracked_allowed.dedup();
    tracked_blocked.sort();
    tracked_blocked.dedup();
    untracked.sort();
    untracked.dedup();

    Dirty {
        tracked_allowed,
        tracked_blocked,
        untracked,
        allowed_paths: allowed.into_iter().map(|path: AllowedPath| path.display).collect(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PorcelainEntry {
    kind: PorcelainKind,
    path: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PorcelainKind {
    Tracked,
    Untracked,
}

fn porcelain_entries(slot_path: &Path) -> Vec<PorcelainEntry> {
    let Ok(output) = crate::cmd::git(
        &crate::cmd::real_cmd,
        Some(slot_path),
        ["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    ) else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }

    let records: Vec<&[u8]> =
        output.stdout.split(|byte| *byte == 0).filter(|record| !record.is_empty()).collect();
    let mut entries = Vec::new();
    let mut idx = 0;
    while idx < records.len() {
        let record = records[idx];
        if record.len() < 4 {
            idx += 1;
            continue;
        }
        let status = &record[..2];
        let path = String::from_utf8_lossy(&record[3..]).into_owned();
        match status {
            b"??" => entries.push(PorcelainEntry {
                kind: PorcelainKind::Untracked,
                path,
            }),
            b"!!" => {}
            _ => {
                entries.push(PorcelainEntry {
                    kind: PorcelainKind::Tracked,
                    path,
                });
                if (matches!(status[0], b'R' | b'C') || matches!(status[1], b'R' | b'C'))
                    && let Some(original) = records.get(idx + 1)
                {
                    entries.push(PorcelainEntry {
                        kind: PorcelainKind::Tracked,
                        path: String::from_utf8_lossy(original).into_owned(),
                    });
                    idx += 1;
                }
            }
        }
        idx += 1;
    }
    entries
}

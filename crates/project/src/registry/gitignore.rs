//! `.gitignore` upkeep for `.emery/`-internal directories. `init`
//! calls [`ensure_gitignore`] to keep the entries current.

use std::fs;
use std::path::Path;

use error::Error;

/// Lines the framework requires in the project `.gitignore`. Both
/// directories are framework-managed and regenerable —
/// `.emery/scratch/` (per-run working state) and the top-level
/// `workspace/` (materialised slots) — and must never be checked in.
///
/// Deliberately **not** listed: `.emery/specs/` (the baseline, the
/// durable system of record) and `.emery/archive/` (the prunable
/// outcome cache). Both stay committable so git history of the baseline
/// — together with the `slice.archive.created` outcome ledger in
/// `journal.jsonl` — is the record of merged work. The adapter cache
/// lives in the out-of-tree OS cache, so it needs no entry here.
const EMERY_GITIGNORE_ENTRIES: &[&str] = &[".emery/scratch/", "workspace/"];

/// Idempotent: ensure each line in `EMERY_GITIGNORE_ENTRIES` appears
/// exactly once (matched with `trim()` per line) in the project
/// `.gitignore`, appending missing lines with a trailing newline.
///
/// Used by `emery init`.
///
/// # Errors
///
/// [`Error::Io`] if the existing `.gitignore` cannot be read, or if the
/// rewritten file cannot be written back.
pub fn ensure_gitignore(project_dir: &Path) -> Result<(), Error> {
    let path = project_dir.join(".gitignore");
    let existing = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(Error::Io(err)),
    };

    let mut updated = existing;
    let mut changed = false;
    for entry in EMERY_GITIGNORE_ENTRIES {
        if updated.lines().any(|line| line.trim() == *entry) {
            continue;
        }
        if !updated.is_empty() && !updated.ends_with('\n') {
            updated.push('\n');
        }
        updated.push_str(entry);
        updated.push('\n');
        changed = true;
    }

    if changed {
        fs::write(&path, updated)?;
    }
    Ok(())
}

//! `.gitignore` upkeep for `.emery/`-internal directories. `init`
//! calls [`ensure_gitignore`] to keep the entries current.

use std::fs;
use std::path::Path;

use error::Error;

/// Lines the framework requires in the project `.gitignore`: the
/// directory is framework-managed and regenerable, never checked in.
///
/// Deliberately **not** listed: `.emery/specs/` (the durable baseline)
/// and `.emery/archive/` (the prunable outcome cache) — both stay
/// committable so git history records merged work.
const EMERY_GITIGNORE_ENTRIES: &[&str] = &[".emery/scratch/"];

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

//! The native fetch leg: resolve a remote locator into an
//! `origin-<nonce>` tree via host `git` or an HTTPS download.

use std::path::{Path, PathBuf};
use std::process::Command;

use error::Error;

use super::is_remote;

/// One fetched origin tree beneath the caller's parent directory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedTree {
    /// The fetched tree: `<parent>/<name>`.
    pub dir: PathBuf,
    /// The tree's directory name (`origin-<nonce>`) — the discard
    /// handle, and the guest-visible segment under the workspaces
    /// mount.
    pub name: String,
    /// The commit the fetch reports; `None` for document origins.
    pub revision: Option<String>,
}

/// Fetch `locator` into a fresh `origin-<nonce>` tree beneath
/// `parent`.
///
/// A Git origin shallow-clones (ambient host credentials, no
/// terminal prompt); any other HTTP(S) locator downloads as a
/// one-file tree. Blocks the calling thread for the duration of the
/// fetch; safe under an async executor (the network leg runs on its
/// own thread).
///
/// # Errors
///
/// - `origin-locator-unsupported` when `locator` is not a remote
///   origin.
/// - `origin-fetch-failed` when the clone or download fails.
pub fn fetch(parent: &Path, locator: &str) -> Result<FetchedTree, Error> {
    if !is_remote(locator) {
        return Err(Error::Diag {
            code: "origin-locator-unsupported",
            detail: format!("`{locator}` is not a Git or HTTPS origin locator"),
        });
    }
    let name = mint_name();
    let dir = parent.join(&name);
    std::fs::create_dir_all(parent).map_err(|source| Error::Filesystem {
        op: "create",
        path: parent.to_path_buf(),
        source,
    })?;

    if git_origin(locator) {
        clone(locator, &dir)?;
        let revision = revision(&dir);
        Ok(FetchedTree { dir, name, revision })
    } else {
        download(locator, &dir)?;
        Ok(FetchedTree {
            dir,
            name,
            revision: None,
        })
    }
}

/// Remove a fetched tree by its `origin-*` name. Idempotent;
/// refuses any name the fetch leg could not have minted.
///
/// # Errors
///
/// - `origin-discard-invalid` for a name outside the `origin-*`
///   grammar.
/// - I/O failures from the removal itself.
pub fn discard(parent: &Path, name: &str) -> Result<(), Error> {
    let minted = name.strip_prefix("origin-").is_some_and(|rest| {
        !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    });
    if !minted {
        return Err(Error::Diag {
            code: "origin-discard-invalid",
            detail: format!("`{name}` is not a fetched origin tree name"),
        });
    }
    let dir = parent.join(name);
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) => Err(Error::Filesystem {
            op: "remove",
            path: dir,
            source,
        }),
    }
}

/// A unique fetch-tree name. Wall-clock nanoseconds plus the process
/// id keep concurrent invocations apart; uniqueness is the only use.
fn mint_name() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    format!("origin-{:x}-{nanos:x}", std::process::id())
}

/// True when `locator` answers as a Git remote. Explicit Git schemes
/// skip the probe; an HTTP(S) locator is probed with `ls-remote`
/// (prompt-free), falling back to the document leg when it refuses.
fn git_origin(locator: &str) -> bool {
    if locator.starts_with("git@") || locator.starts_with("ssh://") {
        return true;
    }
    git(["ls-remote", "--exit-code", locator, "HEAD"]).is_ok()
}

/// Shallow-clone `locator` into `dir`.
fn clone(locator: &str, dir: &Path) -> Result<(), Error> {
    let dest = dir.display().to_string();
    git(["clone", "--depth", "1", "--quiet", locator, &dest]).map_err(|detail| Error::Diag {
        code: "origin-fetch-failed",
        detail: format!("git clone of `{locator}` failed: {detail}"),
    })?;
    Ok(())
}

/// The cloned tree's HEAD commit. Best-effort — a repository that
/// reports none simply carries no `observed-revision`.
fn revision(dir: &Path) -> Option<String> {
    let dir = dir.display().to_string();
    let head = git(["-C", &dir, "rev-parse", "HEAD"]).ok()?;
    let head = head.trim();
    (!head.is_empty()).then(|| head.to_string())
}

/// Run one prompt-free host `git` invocation, capturing stdout.
fn git<'a>(args: impl IntoIterator<Item = &'a str>) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|err| format!("spawning git: {err}"))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Download an HTTP(S) document locator into `dir` as a one-file
/// tree named by the locator's last path segment.
fn download(locator: &str, dir: &Path) -> Result<(), Error> {
    let failed = |detail: String| Error::Diag {
        code: "origin-fetch-failed",
        detail: format!("fetching `{locator}` failed: {detail}"),
    };
    let bytes = get(locator).map_err(failed)?;
    std::fs::create_dir_all(dir).map_err(|source| Error::Filesystem {
        op: "create",
        path: dir.to_path_buf(),
        source,
    })?;
    let file = dir.join(document_name(locator));
    std::fs::write(&file, &bytes).map_err(|source| Error::Filesystem {
        op: "write",
        path: file,
        source,
    })
}

/// One blocking GET on a dedicated thread: `reqwest::blocking`
/// refuses to run on an async executor's worker, and callers may sit
/// on one.
fn get(locator: &str) -> Result<Vec<u8>, String> {
    let locator = locator.to_string();
    std::thread::spawn(move || {
        let response = reqwest::blocking::get(&locator).map_err(|err| err.to_string())?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("origin answered {status}"));
        }
        Ok(response.bytes().map_err(|err| err.to_string())?.to_vec())
    })
    .join()
    .map_err(|_panic| "download thread panicked".to_string())?
}

/// The document filename a locator materializes as: its last path
/// segment restricted to a portable charset, else `document`.
fn document_name(locator: &str) -> String {
    let path = locator.split(['?', '#']).next().unwrap_or_default();
    let segment = path.trim_end_matches('/').rsplit('/').next().unwrap_or_default();
    let clean: String = segment
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        .collect();
    if clean.is_empty() || clean.trim_matches('.').is_empty() {
        "document".to_string()
    } else {
        clean
    }
}

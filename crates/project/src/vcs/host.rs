//! The native fetch leg behind the `emery:vcs/trees` seam: stage one
//! locator into a `vcs-<nonce>` tree beneath the staging root via
//! host `git` or an HTTPS download. Policy stays with the caller.

use std::path::{Path, PathBuf};
use std::process::Command;

use error::Error;

use super::is_remote;
use crate::binding::{Location, Locator, Meter, Policy, checkout, fetch_https};
use crate::seam::{TreeCredentials, TreeError, TreeLimits};

/// One staged tree beneath the caller's staging root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchedTree {
    /// The staged tree: `<staging>/<name>`.
    pub dir: PathBuf,
    /// The tree's directory name (`vcs-<nonce>`) — the discard
    /// handle, and the guest-visible segment under the staging mount.
    pub name: String,
    /// The commit the fetch reports; `None` for document origins.
    pub revision: Option<String>,
}

/// Stage `locator` into a fresh `vcs-<nonce>` tree beneath `staging`.
///
/// [`TreeCredentials::Ambient`] is the RFC-104 archaeology leg: a Git
/// origin shallow-clones with the operator's ambient host credentials
/// (no terminal prompt); any other HTTP(S) locator downloads as a
/// one-file tree. [`TreeCredentials::None`] is the RFC-88 bind leg:
/// a hardened exact-revision checkout (no hooks, submodules, LFS, or
/// prompts) or a bounded, gated HTTPS document fetch under `limits`.
///
/// # Errors
///
/// [`TreeError::InvalidRequest`] for an unfetchable locator,
/// [`TreeError::Access`] when the origin refuses or cannot be
/// reached, [`TreeError::Limit`] when a transport bound is exhausted.
pub fn fetch(
    staging: &Path, locator: &str, credentials: TreeCredentials, limits: &TreeLimits,
) -> Result<FetchedTree, TreeError> {
    std::fs::create_dir_all(staging).map_err(|err| {
        TreeError::Internal(format!("creating staging root {}: {err}", staging.display()))
    })?;
    match credentials {
        TreeCredentials::Ambient => ambient(staging, locator, limits),
        TreeCredentials::None => hardened(staging, locator, limits),
    }
}

/// The ambient-credential archaeology leg (RFC-104 system survey),
/// metered like the hardened leg (D11): document bytes are bounded
/// during the read, clone bytes are charged after the shallow clone,
/// and both share the wall-clock budget. A failed or over-limit fetch
/// never leaves a partial staged tree behind.
fn ambient(staging: &Path, locator: &str, limits: &TreeLimits) -> Result<FetchedTree, TreeError> {
    if !is_remote(locator) {
        return Err(TreeError::InvalidRequest(format!(
            "`{locator}` is not a Git or HTTPS origin locator"
        )));
    }
    let policy = policy_from(limits);
    let mut meter = Meter::new();
    let name = mint_name();
    let dir = staging.join(&name);
    let fetched = if git_origin(locator) {
        clone(locator, &dir).and_then(|()| charge_tree(&dir, &mut meter, &policy)).map(|()| {
            FetchedTree {
                dir: dir.clone(),
                name: name.clone(),
                revision: revision(&dir),
            }
        })
    } else {
        download(locator, &dir, &policy, &mut meter).map(|()| FetchedTree {
            dir: dir.clone(),
            name: name.clone(),
            revision: None,
        })
    };
    if fetched.is_err() {
        // A partial tree never survives its failed fetch (D11).
        drop(std::fs::remove_dir_all(&dir));
    }
    fetched
}

/// Charge every staged file's bytes against the fetch budget — the
/// post-clone tree charge; a shallow clone cannot be bounded
/// mid-transfer, so an oversize tree is discarded right after.
fn charge_tree(dir: &Path, meter: &mut Meter, policy: &Policy) -> Result<(), TreeError> {
    let mut total = 0_u64;
    let mut pending = vec![dir.to_path_buf()];
    while let Some(current) = pending.pop() {
        let entries = std::fs::read_dir(&current)
            .map_err(|err| TreeError::Internal(format!("listing {}: {err}", current.display())))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.is_dir() {
                pending.push(path);
            } else if meta.is_file() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    meter.bytes(total, policy).map_err(|err| classify(&err))
}

/// The hardened credential-free bind leg (RFC-88 D9 delivery bind).
fn hardened(staging: &Path, locator: &str, limits: &TreeLimits) -> Result<FetchedTree, TreeError> {
    let location = Location::parse(locator, None).map_err(|err| classify(&err))?;
    let policy = policy_from(limits);
    let mut meter = Meter::new();
    match &location.locator {
        Locator::Git { url, revision } => {
            let name = mint_name();
            let dir = staging.join(&name);
            let sha = checkout(url, revision, &dir, &policy, &mut meter).map_err(|err| {
                // A partial tree never survives its failed fetch (D11).
                drop(std::fs::remove_dir_all(&dir));
                classify(&err)
            })?;
            Ok(FetchedTree {
                dir,
                name,
                revision: Some(sha),
            })
        }
        Locator::Https(url) => {
            let bytes = fetch_https(url, &policy, &mut meter).map_err(|err| classify(&err))?;
            let name = mint_name();
            let dir = staging.join(&name);
            std::fs::create_dir_all(&dir).map_err(|err| {
                TreeError::Internal(format!("creating staged tree {}: {err}", dir.display()))
            })?;
            let file = dir.join(document_name(url));
            std::fs::write(&file, &bytes)
                .map_err(|err| TreeError::Internal(format!("writing {}: {err}", file.display())))?;
            Ok(FetchedTree {
                dir,
                name,
                revision: None,
            })
        }
        Locator::Path(_) => Err(TreeError::InvalidRequest(format!(
            "`{locator}` is a local path; path locators are read in-process, never fetched"
        ))),
    }
}

/// The transport slice of a bind policy: only the bounds the seam
/// hands down; wave-level budgets stay engine-side.
fn policy_from(limits: &TreeLimits) -> Policy {
    Policy {
        concurrency: 1,
        bindings: usize::MAX,
        api_requests: usize::MAX,
        time_ms: limits.time_ms,
        inspected_bytes: limits.max_bytes,
        imported_trees: usize::MAX,
        https_redirects: usize::try_from(limits.max_redirects).unwrap_or(usize::MAX),
        https_body: usize::try_from(limits.max_bytes).unwrap_or(usize::MAX),
    }
}

/// Map a kernel diagnostic onto the seam's closed tree-error taxonomy.
/// The full `code: detail` rendering survives in the variant payload.
fn classify(err: &Error) -> TreeError {
    let detail = err.to_string();
    let Error::Diag { code, .. } = err else {
        return TreeError::Internal(detail);
    };
    match *code {
        "binding-budget-exhausted" | "https-redirect-limit" | "https-body-limit" => {
            TreeError::Limit(detail)
        }
        "locator-malformed" | "locator-http-unsupported" | "locator-credentials-forbidden" => {
            TreeError::InvalidRequest(detail)
        }
        "git-ingest-failed"
        | "git-revision-unavailable"
        | "https-fetch-failed"
        | "locator-private-network" => TreeError::Access(detail),
        _ => TreeError::Internal(detail),
    }
}

/// Remove a staged tree by its `vcs-*` name. Idempotent; refuses any
/// name the fetch leg could not have minted.
///
/// # Errors
///
/// [`TreeError::InvalidRequest`] for a name outside the `vcs-*`
/// grammar; [`TreeError::Internal`] for removal I/O failures.
pub fn discard(staging: &Path, name: &str) -> Result<(), TreeError> {
    let minted = name.strip_prefix("vcs-").is_some_and(|rest| {
        !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    });
    if !minted {
        return Err(TreeError::InvalidRequest(format!("`{name}` is not a staged tree name")));
    }
    let dir = staging.join(name);
    match std::fs::remove_dir_all(&dir) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(TreeError::Internal(format!("removing {}: {err}", dir.display()))),
    }
}

/// Best-effort age-based sweep of abandoned staging trees.
///
/// Removes every `vcs-*` entry whose modification time is older than
/// `max_age`. Errors are swallowed — a live fetch's tree is younger
/// by construction, and the next sweep retries.
pub fn sweep_stale(staging: &Path, max_age: std::time::Duration) {
    let Ok(entries) = std::fs::read_dir(staging) else {
        return;
    };
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with("vcs-") {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .ok()
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age > max_age);
        if stale {
            drop(std::fs::remove_dir_all(entry.path()));
        }
    }
}

/// A unique staged-tree name. Wall-clock nanoseconds plus the process
/// id keep concurrent invocations apart; uniqueness is the only use.
fn mint_name() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    format!("vcs-{:x}-{nanos:x}", std::process::id())
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

/// Shallow-clone `locator` into `dir` with ambient host credentials.
fn clone(locator: &str, dir: &Path) -> Result<(), TreeError> {
    let dest = dir.display().to_string();
    git(["clone", "--depth", "1", "--quiet", locator, &dest]).map_err(|detail| {
        TreeError::Access(format!("git clone of `{locator}` failed: {detail}"))
    })?;
    Ok(())
}

/// The cloned tree's HEAD commit. Best-effort — a repository that
/// reports none simply carries no observed revision.
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
/// tree named by the locator's last path segment, bounded by the
/// fetch budget's body cap.
fn download(
    locator: &str, dir: &Path, policy: &Policy, meter: &mut Meter,
) -> Result<(), TreeError> {
    let failed =
        |detail: String| TreeError::Access(format!("fetching `{locator}` failed: {detail}"));
    let cap = policy.https_body;
    let bytes = get(locator, cap.saturating_add(1)).map_err(failed)?;
    if bytes.len() > cap {
        return Err(TreeError::Limit(format!(
            "document `{locator}` exceeds the {cap}-byte fetch budget"
        )));
    }
    meter
        .bytes(u64::try_from(bytes.len()).unwrap_or(u64::MAX), policy)
        .map_err(|err| classify(&err))?;
    std::fs::create_dir_all(dir).map_err(|err| {
        TreeError::Internal(format!("creating staged tree {}: {err}", dir.display()))
    })?;
    let file = dir.join(document_name(locator));
    std::fs::write(&file, &bytes)
        .map_err(|err| TreeError::Internal(format!("writing {}: {err}", file.display())))
}

/// One blocking GET on a dedicated thread, reading at most `cap`
/// bytes: `reqwest::blocking` refuses to run on an async executor's
/// worker, and callers may sit on one.
fn get(locator: &str, cap: usize) -> Result<Vec<u8>, String> {
    use std::io::Read as _;
    let locator = locator.to_string();
    std::thread::spawn(move || {
        let response = reqwest::blocking::get(&locator).map_err(|err| err.to_string())?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("origin answered {status}"));
        }
        let mut bytes = Vec::new();
        response
            .take(u64::try_from(cap).unwrap_or(u64::MAX))
            .read_to_end(&mut bytes)
            .map_err(|err| err.to_string())?;
        Ok(bytes)
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

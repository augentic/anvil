//! Exact-revision Git checkout: no hooks, submodules, LFS, or prompts.

use std::path::Path;
use std::process::{Command, Output};

use error::Error;
use project::binding::{Locator, Meter, Policy};

/// Checkout result: exact SHA plus an optional moved-branch warning.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exact {
    /// Resolved commit id.
    pub revision: String,
    /// Set when `prior` no longer matches the named ref's tip.
    pub warning: Option<String>,
}

/// Clone `url` at `revision` into `dest` with hooks, LFS, and submodules off.
///
/// Mutable refs resolve via `ls-remote`. A `prior` SHA that still exists but
/// is no longer the named ref's tip yields a freshness warning; the checkout
/// uses `prior`. An unavailable recorded commit is an error.
///
/// # Errors
///
/// `git-ingest-failed`, `git-revision-unavailable`, `binding-budget-exhausted`.
pub fn checkout(
    url: &str, revision: &str, prior: Option<&str>, dest: &Path, policy: &Policy, meter: &mut Meter,
) -> Result<Exact, Error> {
    let (sha, warning) = resolve_sha(url, revision, prior, policy, meter)?;
    if dest.exists() {
        std::fs::remove_dir_all(dest)?;
    }
    meter.api(policy)?;
    run(
        &[
            "clone",
            "--no-checkout",
            "--no-hardlinks",
            "--recurse-submodules=no",
            "--config",
            "core.hooksPath=/dev/null",
            "--config",
            "submodule.recurse=false",
            "--config",
            "protocol.file.allow=always",
            "--config",
            "init.templateDir=",
            url,
            dest.to_str().ok_or_else(|| failed("checkout path is not UTF-8"))?,
        ],
        None,
    )?;
    meter.api(policy)?;
    match run(&["fetch", "--force", "--no-tags", "origin", &sha], Some(dest)) {
        Ok(()) => {}
        Err(_) => {
            return Err(Error::Diag {
                code: "git-revision-unavailable",
                detail: format!("recorded commit `{sha}` is not available at `{url}`"),
            });
        }
    }
    run(&["checkout", "--force", "--recurse-submodules=no", &sha], Some(dest))?;
    Ok(Exact {
        revision: sha,
        warning,
    })
}

fn resolve_sha(
    url: &str, revision: &str, prior: Option<&str>, policy: &Policy, meter: &mut Meter,
) -> Result<(String, Option<String>), Error> {
    if Locator::is_sha(revision) {
        return Ok((revision.to_string(), None));
    }
    meter.api(policy)?;
    let tip = ls_remote(url, revision)?;
    if let Some(prior) = prior {
        if prior != tip {
            return Ok((
                prior.to_string(),
                Some(format!(
                    "git ref `{revision}` moved from `{prior}` to `{tip}`; ingesting recorded commit"
                )),
            ));
        }
        return Ok((prior.to_string(), None));
    }
    Ok((tip, None))
}

fn ls_remote(url: &str, revision: &str) -> Result<String, Error> {
    let output = command(&["ls-remote", url, revision], None)?;
    if !output.status.success() {
        return Err(failed(&format!(
            "git ls-remote `{url}` `{revision}` failed: {}",
            stderr(&output)
        )));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let sha = stdout
        .lines()
        .find_map(|line| {
            let (sha, _) = line.split_once('\t')?;
            Locator::is_sha(sha.trim()).then(|| sha.trim().to_string())
        })
        .ok_or_else(|| Error::Diag {
            code: "git-revision-unavailable",
            detail: format!("git ref `{revision}` is not available at `{url}`"),
        })?;
    Ok(sha)
}

fn run(args: &[&str], cwd: Option<&Path>) -> Result<(), Error> {
    let output = command(args, cwd)?;
    if output.status.success() {
        return Ok(());
    }
    Err(failed(&format!("git {} failed: {}", args[0], stderr(&output))))
}

fn command(args: &[&str], cwd: Option<&Path>) -> Result<Output, Error> {
    let mut cmd = Command::new("git");
    cmd.args(["-c", "core.hooksPath=/dev/null", "-c", "submodule.recurse=false"]);
    cmd.args(args);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    cmd.env("GIT_CONFIG_GLOBAL", devnull());
    cmd.env("GIT_CONFIG_SYSTEM", devnull());
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.env("GIT_LFS_SKIP_SMUDGE", "1");
    cmd.env_remove("GIT_ASKPASS");
    cmd.output().map_err(|err| failed(&format!("failed to spawn git: {err}")))
}

fn stderr(output: &Output) -> String {
    let text = String::from_utf8_lossy(&output.stderr);
    text.trim().lines().next().unwrap_or("git command failed").to_string()
}

fn failed(detail: &str) -> Error {
    Error::Diag {
        code: "git-ingest-failed",
        detail: detail.into(),
    }
}

const fn devnull() -> &'static str {
    #[cfg(windows)]
    {
        "NUL"
    }
    #[cfg(not(windows))]
    {
        "/dev/null"
    }
}

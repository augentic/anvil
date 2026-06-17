//! `git` invocations for fetching a adapter from a GitHub URI.
//! Sparse-checkouts the adapter's parent directory rather than the
//! full repository so the cache stays cheap on large monorepos. The
//! shared codex tree (`adapters/shared/rules/`) is fetched in the same
//! sparse set so codex distribution (RM-07) can copy it from the same
//! checkout without a second clone.

use specify_error::Error;
use tempfile::TempDir;

use crate::cmd;

/// Shared codex tree path, fetched alongside the adapter so
/// `cache_codex` can distribute `UNI-*` / `CORE-*` packs from the same
/// checkout. Harmless when the source repo carries no such tree —
/// `sparse-checkout set` simply materialises nothing for it.
const SHARED_RULES_PATH: &str = "adapters/shared/rules";
/// Shared spec runtime mirror (symlinks into `plugins/spec/references/`).
const SHARED_RUNTIME_PATH: &str = "adapters/shared/references";
/// Plugin references tree symlink targets for sparse GitHub checkouts.
const PLUGINS_SPEC_REFERENCES_PATH: &str = "plugins/spec/references";

pub(super) fn sparse_checkout_github(
    repo_url: &str, checkout_ref: Option<&str>, adapter_path: &str,
) -> Result<TempDir, Error> {
    let checkout = tempfile::Builder::new().prefix("specify-checkout-").tempdir()?;
    let checkout_arg = checkout.path().to_string_lossy().to_string();

    let mut clone_args = vec!["clone", "--depth", "1", "--filter=blob:none", "--sparse"];
    if let Some(reference) = checkout_ref {
        clone_args.push("--branch");
        clone_args.push(reference);
    }
    clone_args.push(repo_url);
    clone_args.push(&checkout_arg);
    run_git(&clone_args, "clone adapter repository")?;

    let sparse_path = sparse_checkout_path(adapter_path);
    run_git(
        &[
            "-C",
            &checkout_arg,
            "sparse-checkout",
            "set",
            "--",
            sparse_path,
            SHARED_RULES_PATH,
            SHARED_RUNTIME_PATH,
            PLUGINS_SPEC_REFERENCES_PATH,
        ],
        "sparse-checkout adapter, shared-codex, and spec-runtime paths",
    )?;
    Ok(checkout)
}

fn sparse_checkout_path(adapter_path: &str) -> &str {
    match adapter_path.rsplit_once('/') {
        Some((parent, _name)) if !parent.is_empty() => parent,
        _ => adapter_path,
    }
}

fn run_git(args: &[&str], action: &str) -> Result<(), Error> {
    cmd::git_checked(&cmd::real_cmd, None, args).map(|_output| ()).map_err(
        |failure| match failure {
            cmd::GitFailure::Spawn(err) => Error::Diag {
                code: "adapter-git-spawn-failed",
                detail: format!("failed to spawn `git` to {action}: {err}"),
            },
            cmd::GitFailure::Exit { stderr } => Error::Diag {
                code: "adapter-git-failed",
                detail: format!("git failed to {action}: {stderr}"),
            },
        },
    )
}

#[cfg(test)]
mod tests;

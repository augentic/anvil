use std::path::Path;

use anyhow::{Context, Result, bail};

/// Resolve git credentials from SSH agent or GITHUB_TOKEN env var.
fn credentials_callback(
    _url: &str, username_from_url: Option<&str>, allowed_types: git2::CredentialType,
) -> std::result::Result<git2::Cred, git2::Error> {
    if allowed_types.contains(git2::CredentialType::SSH_KEY) {
        let user = username_from_url.unwrap_or("git");
        return git2::Cred::ssh_key_from_agent(user);
    }

    if allowed_types.contains(git2::CredentialType::USER_PASS_PLAINTEXT)
        && let Ok(token) = std::env::var("GITHUB_TOKEN")
    {
        return git2::Cred::userpass_plaintext("x-access-token", &token);
    }

    Err(git2::Error::from_str("no suitable credentials found"))
}

fn remote_callbacks() -> git2::RemoteCallbacks<'static> {
    let mut callbacks = git2::RemoteCallbacks::new();
    callbacks.credentials(credentials_callback);
    callbacks
}

/// Clone a repo with depth=1 into `dest`.
pub async fn clone_shallow(repo_url: &str, dest: &Path) -> Result<()> {
    let url = repo_url.to_string();
    let dest = dest.to_path_buf();
    tokio::task::spawn_blocking(move || clone_shallow_sync(&url, &dest))
        .await
        .context("join clone task")?
}

fn clone_shallow_sync(repo_url: &str, dest: &Path) -> Result<()> {
    let mut fo = git2::FetchOptions::new();
    fo.remote_callbacks(remote_callbacks());
    fo.depth(1);

    git2::build::RepoBuilder::new()
        .fetch_options(fo)
        .clone(repo_url, dest)
        .with_context(|| format!("cloning {repo_url}"))?;
    Ok(())
}

/// Check whether a branch exists locally or on the remote.
pub async fn branch_exists(repo_dir: &Path, branch: &str) -> Result<bool> {
    let dir = repo_dir.to_path_buf();
    let branch = branch.to_string();
    tokio::task::spawn_blocking(move || {
        let repo = git2::Repository::open(&dir)
            .with_context(|| format!("opening repo at {}", dir.display()))?;
        let local = format!("refs/heads/{branch}");
        let remote = format!("refs/remotes/origin/{branch}");
        Ok(repo.find_reference(&local).is_ok() || repo.find_reference(&remote).is_ok())
    })
    .await
    .context("join branch-exists task")?
}

/// Create and check out a new branch at HEAD.
pub async fn checkout_new_branch(repo_dir: &Path, branch: &str) -> Result<()> {
    let dir = repo_dir.to_path_buf();
    let branch = branch.to_string();
    tokio::task::spawn_blocking(move || checkout_new_branch_sync(&dir, &branch))
        .await
        .context("join checkout-new task")?
}

fn checkout_new_branch_sync(repo_dir: &Path, branch: &str) -> Result<()> {
    let repo = git2::Repository::open(repo_dir)
        .with_context(|| format!("opening repo at {}", repo_dir.display()))?;
    let head = repo.head().context("reading HEAD")?.peel_to_commit().context("peeling HEAD to commit")?;
    repo.branch(branch, &head, false)
        .with_context(|| format!("creating branch '{branch}'"))?;
    let refname = format!("refs/heads/{branch}");
    repo.set_head(&refname)
        .with_context(|| format!("setting HEAD to '{refname}'"))?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .context("checkout new branch")?;
    Ok(())
}

/// Check out an existing branch.
pub async fn checkout_existing_branch(repo_dir: &Path, branch: &str) -> Result<()> {
    let dir = repo_dir.to_path_buf();
    let branch = branch.to_string();
    tokio::task::spawn_blocking(move || checkout_existing_sync(&dir, &branch))
        .await
        .context("join checkout-existing task")?
}

fn checkout_existing_sync(repo_dir: &Path, branch: &str) -> Result<()> {
    let repo = git2::Repository::open(repo_dir)
        .with_context(|| format!("opening repo at {}", repo_dir.display()))?;
    let refname = format!("refs/heads/{branch}");

    if repo.find_reference(&refname).is_ok() {
        repo.set_head(&refname)
            .with_context(|| format!("setting HEAD to '{refname}'"))?;
    } else {
        let remote_ref = format!("refs/remotes/origin/{branch}");
        let reference = repo.find_reference(&remote_ref)
            .with_context(|| format!("branch '{branch}' not found locally or in origin"))?;
        let commit = reference.peel_to_commit()
            .with_context(|| format!("peeling remote ref '{remote_ref}' to commit"))?;
        repo.branch(branch, &commit, false)
            .with_context(|| format!("creating local branch '{branch}'"))?;
        repo.set_head(&format!("refs/heads/{branch}"))
            .with_context(|| format!("setting HEAD to '{branch}'"))?;
    }

    repo.checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
        .with_context(|| format!("checking out branch '{branch}'"))?;
    Ok(())
}

/// Stage all changes, commit, and push to the remote branch.
pub async fn add_commit_push(repo_dir: &Path, message: &str, branch: &str) -> Result<()> {
    let dir = repo_dir.to_path_buf();
    let msg = message.to_string();
    let branch = branch.to_string();
    tokio::task::spawn_blocking(move || add_commit_push_sync(&dir, &msg, &branch))
        .await
        .context("join commit-push task")?
}

fn add_commit_push_sync(repo_dir: &Path, message: &str, branch: &str) -> Result<()> {
    let repo = git2::Repository::open(repo_dir)
        .with_context(|| format!("opening repo at {}", repo_dir.display()))?;

    let mut index = repo.index().context("reading index")?;
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .context("staging files")?;
    index.write().context("writing index")?;

    let tree_oid = index.write_tree().context("writing tree")?;
    let tree = repo.find_tree(tree_oid).context("finding tree")?;

    let head = repo.head().context("reading HEAD")?.peel_to_commit().context("peeling HEAD to commit")?;
    let sig = repo
        .signature()
        .or_else(|_| git2::Signature::now("alc", "alc@augentic.io"))
        .context("creating signature")?;

    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
        .context("creating commit")?;

    let mut remote = repo.find_remote("origin").context("finding remote 'origin'")?;
    let refspec = format!("refs/heads/{branch}:refs/heads/{branch}");
    let mut push_opts = git2::PushOptions::new();
    push_opts.remote_callbacks(remote_callbacks());
    remote
        .push(&[&refspec], Some(&mut push_opts))
        .with_context(|| format!("pushing to origin/{branch}"))?;

    Ok(())
}

/// Parse a GitHub PR URL into (owner, repo, pr_number).
pub fn parse_pr_url(url: &str) -> Result<(String, String, u64)> {
    let stripped = url
        .trim_end_matches('/')
        .strip_prefix("https://github.com/")
        .with_context(|| format!("not a GitHub PR URL: {url}"))?;
    let parts: Vec<&str> = stripped.split('/').collect();
    if parts.len() < 4 || parts[2] != "pull" {
        bail!("unexpected PR URL format: {url}");
    }
    let owner = parts[0].to_string();
    let repo = parts[1].to_string();
    let number: u64 = parts[3]
        .parse()
        .with_context(|| format!("invalid PR number in URL: {url}"))?;
    Ok((owner, repo, number))
}

/// Extract the default branch name from a remote repo.
pub async fn default_branch(repo_dir: &Path) -> Result<String> {
    let dir = repo_dir.to_path_buf();
    tokio::task::spawn_blocking(move || default_branch_sync(&dir))
        .await
        .context("join default-branch task")?
}

fn default_branch_sync(repo_dir: &Path) -> Result<String> {
    let repo = git2::Repository::open(repo_dir)
        .with_context(|| format!("opening repo at {}", repo_dir.display()))?;
    let remote = repo.find_remote("origin").context("finding remote 'origin'")?;
    let head = remote.default_branch()?;
    let refname = head.as_str().context("non-utf8 default branch")?;
    Ok(refname
        .strip_prefix("refs/heads/")
        .unwrap_or(refname)
        .to_string())
}

/// Derive owner/repo from a clone URL for GitHub API calls.
pub fn parse_repo_url(url: &str) -> Result<(String, String)> {
    let cleaned = url.trim_end_matches(".git");
    if let Some(rest) = cleaned.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }
    if let Some(rest) = cleaned.strip_prefix("git@github.com:") {
        let parts: Vec<&str> = rest.splitn(2, '/').collect();
        if parts.len() == 2 {
            return Ok((parts[0].to_string(), parts[1].to_string()));
        }
    }
    bail!("cannot parse owner/repo from URL: {url}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pr_url_valid() {
        let (owner, repo, num) =
            parse_pr_url("https://github.com/augentic/lifecycle/pull/42").unwrap();
        assert_eq!(owner, "augentic");
        assert_eq!(repo, "lifecycle");
        assert_eq!(num, 42);
    }

    #[test]
    fn parse_pr_url_trailing_slash() {
        let (owner, repo, num) =
            parse_pr_url("https://github.com/org/repo/pull/7/").unwrap();
        assert_eq!(owner, "org");
        assert_eq!(repo, "repo");
        assert_eq!(num, 7);
    }

    #[test]
    fn parse_pr_url_invalid() {
        assert!(parse_pr_url("https://gitlab.com/org/repo/pull/1").is_err());
        assert!(parse_pr_url("not-a-url").is_err());
    }

    #[test]
    fn parse_repo_url_ssh() {
        let (owner, repo) =
            parse_repo_url("git@github.com:augentic/lifecycle.git").unwrap();
        assert_eq!(owner, "augentic");
        assert_eq!(repo, "lifecycle");
    }

    #[test]
    fn parse_repo_url_https() {
        let (owner, repo) =
            parse_repo_url("https://github.com/augentic/lifecycle").unwrap();
        assert_eq!(owner, "augentic");
        assert_eq!(repo, "lifecycle");
    }
}

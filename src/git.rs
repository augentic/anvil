use std::path::Path;
use std::process::{Command, Output};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

fn check_output(output: Output, context: &str) -> Result<String> {
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{context}: {stderr}");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run an arbitrary command in a directory and check for success.
pub fn run_cmd(program: &str, args: &[&str], dir: &Path) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .with_context(|| format!("spawning {program}"))?;
    check_output(output, &format!("{program} {}", args.join(" ")))
}

pub fn clone_shallow(repo_url: &str, dest: &Path) -> Result<()> {
    let output = Command::new("git")
        .args(["clone", "--depth=1", repo_url])
        .arg(dest)
        .output()
        .context("spawning git clone")?;
    check_output(output, "git clone")?;
    Ok(())
}

pub fn checkout_new_branch(repo_dir: &Path, branch: &str) -> Result<()> {
    run_cmd("git", &["checkout", "-b", branch], repo_dir)?;
    Ok(())
}

pub fn add_commit_push(repo_dir: &Path, message: &str, branch: &str) -> Result<()> {
    run_cmd("git", &["add", "-A"], repo_dir)?;
    run_cmd("git", &["commit", "-m", message], repo_dir)?;
    run_cmd("git", &["push", "-u", "origin", branch], repo_dir)?;
    Ok(())
}

pub fn create_draft_pr(repo_dir: &Path, title: &str, body: &str) -> Result<String> {
    run_cmd(
        "gh",
        &["pr", "create", "--draft", "--title", title, "--body", body],
        repo_dir,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequestInfo {
    pub url: String,
    pub is_draft: bool,
    pub state: String,
    pub merged_at: Option<String>,
}

pub fn pull_request_info(pr_url: &str, dir: &Path) -> Result<PullRequestInfo> {
    let output = run_cmd(
        "gh",
        &["pr", "view", pr_url, "--json", "url,isDraft,state,mergedAt"],
        dir,
    )?;
    serde_json::from_str(&output)
        .with_context(|| format!("parsing gh pr view json for {pr_url}"))
}

pub fn mark_pr_ready(pr_url: &str, dir: &Path) -> Result<()> {
    run_cmd("gh", &["pr", "ready", pr_url], dir)?;
    Ok(())
}

use anyhow::{Context, Result};
use serde::Deserialize;

/// PR metadata returned by the GitHub API.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct PullRequestInfo {
    pub url: String,
    pub is_draft: bool,
    pub state: String,
    pub merged_at: Option<String>,
}

/// Create a draft pull request, returning the HTML URL.
pub async fn create_draft_pr(
    client: &octocrab::Octocrab,
    owner: &str, repo: &str, head: &str, base: &str, title: &str, body: &str,
) -> Result<String> {
    let pr = client
        .pulls(owner, repo)
        .create(title, head, base)
        .body(body)
        .draft(Some(true))
        .send()
        .await
        .with_context(|| format!("creating draft PR in {owner}/{repo}"))?;

    pr.html_url
        .map(|u| u.to_string())
        .context("PR response missing html_url")
}

/// Fetch PR info by number.
pub async fn pull_request_info(
    client: &octocrab::Octocrab, owner: &str, repo: &str, number: u64,
) -> Result<PullRequestInfo> {
    let pr = client
        .pulls(owner, repo)
        .get(number)
        .await
        .with_context(|| format!("fetching PR #{number} in {owner}/{repo}"))?;

    let state = pr.state.map_or_else(
        || "unknown".to_string(),
        |s| match s {
            octocrab::models::IssueState::Open => "OPEN".to_string(),
            octocrab::models::IssueState::Closed => "CLOSED".to_string(),
            other => format!("{other:?}").to_uppercase(),
        },
    );

    let merged_at = pr.merged_at.map(|t| t.to_rfc3339());

    Ok(PullRequestInfo {
        url: pr
            .html_url
            .map(|u| u.to_string())
            .unwrap_or_default(),
        is_draft: pr.draft.unwrap_or(false),
        state,
        merged_at,
    })
}

/// Mark a draft PR as ready for review via the GraphQL API.
pub async fn mark_pr_ready(
    client: &octocrab::Octocrab, owner: &str, repo: &str, number: u64,
) -> Result<()> {
    let pr = client
        .pulls(owner, repo)
        .get(number)
        .await
        .with_context(|| format!("fetching PR #{number} for ready mutation"))?;

    let node_id = pr
        .node_id
        .filter(|id| !id.is_empty())
        .with_context(|| format!("PR #{number} in {owner}/{repo} has no node_id for GraphQL mutation"))?;

    let query = format!(
        r#"mutation {{ markPullRequestReadyForReview(input: {{ pullRequestId: "{node_id}" }}) {{ pullRequest {{ isDraft }} }} }}"#
    );

    client
        .graphql::<serde_json::Value>(&query)
        .await
        .with_context(|| format!("marking PR #{number} ready in {owner}/{repo}"))?;

    Ok(())
}

/// Enable GitHub's native auto-merge (squash) on a PR via the GraphQL API.
///
/// Requires the repo to have branch protection rules configured. The mutation
/// is idempotent — calling it when auto-merge is already enabled is a no-op.
pub async fn enable_auto_merge(
    client: &octocrab::Octocrab, owner: &str, repo: &str, number: u64,
) -> Result<()> {
    let pr = client
        .pulls(owner, repo)
        .get(number)
        .await
        .with_context(|| format!("fetching PR #{number} for auto-merge mutation"))?;

    let node_id = pr
        .node_id
        .filter(|id| !id.is_empty())
        .with_context(|| {
            format!("PR #{number} in {owner}/{repo} has no node_id for GraphQL mutation")
        })?;

    let query = format!(
        r#"mutation {{ enablePullRequestAutoMerge(input: {{ pullRequestId: "{node_id}", mergeMethod: SQUASH }}) {{ pullRequest {{ autoMergeRequest {{ enabledAt }} }} }} }}"#
    );

    client
        .graphql::<serde_json::Value>(&query)
        .await
        .with_context(|| format!("enabling auto-merge on PR #{number} in {owner}/{repo}"))?;

    Ok(())
}

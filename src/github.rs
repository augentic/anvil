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

fn build_client() -> Result<octocrab::Octocrab> {
    let token = std::env::var("GITHUB_TOKEN")
        .context("GITHUB_TOKEN env var required for GitHub API access")?;
    octocrab::Octocrab::builder()
        .personal_token(token)
        .build()
        .context("building GitHub client")
}

/// Create a draft pull request, returning the HTML URL.
pub async fn create_draft_pr(
    owner: &str, repo: &str, head: &str, base: &str, title: &str, body: &str,
) -> Result<String> {
    let client = build_client()?;
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
pub async fn pull_request_info(owner: &str, repo: &str, number: u64) -> Result<PullRequestInfo> {
    let client = build_client()?;
    let pr = client
        .pulls(owner, repo)
        .get(number)
        .await
        .with_context(|| format!("fetching PR #{number} in {owner}/{repo}"))?;

    let state = pr.state.map_or_else(
        || "unknown".to_string(),
        |s| format!("{s:?}").to_uppercase(),
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
pub async fn mark_pr_ready(owner: &str, repo: &str, number: u64) -> Result<()> {
    let client = build_client()?;

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

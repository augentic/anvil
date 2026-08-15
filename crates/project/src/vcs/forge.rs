//! The native GitHub REST `forge.find` kernel (RFC-95 D10), run
//! in-process by the native provider and the launcher backend.

use std::process::Command;

use crate::seam::{ForgeError, PrState, PullRequest};

/// One forge read's configuration: the REST base (the production
/// GitHub API, or a local fixture in tests) and an optional token.
/// The token lives only here — never logged, never in an error.
#[derive(Clone, Debug)]
pub struct Config {
    /// REST base URL, `https://api.github.com` in production.
    pub api_base: String,
    /// Bearer token, when one resolved.
    pub token: Option<String>,
}

impl Config {
    /// The production GitHub configuration with the D10 token order:
    /// `GITHUB_TOKEN`, else `gh auth token`, else unauthenticated.
    /// Called once per find at the composition edge, never in kernels.
    #[must_use]
    pub fn github() -> Self {
        Self {
            api_base: "https://api.github.com".to_string(),
            token: resolve_token(),
        }
    }
}

/// `GITHUB_TOKEN` from the environment, else `gh auth token` when the
/// CLI is present, else `None` (unauthenticated — sufficient for
/// public repositories).
fn resolve_token() -> Option<String> {
    if let Ok(token) = std::env::var("GITHUB_TOKEN")
        && !token.trim().is_empty()
    {
        return Some(token.trim().to_string());
    }
    let output = Command::new("gh").args(["auth", "token"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let token = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!token.is_empty()).then_some(token)
}

/// Every open, merged, and closed pull request for
/// `(repository, branch)`, pagination followed to exhaustion.
///
/// The zero / one / several rule, trailer matching, and `merged-at`
/// ordering are engine checks over these results.
///
/// # Errors
///
/// `InvalidRequest` for a non-GitHub repository reference, `Auth` on
/// 401/403, `Transport` on connection failures and other statuses.
pub fn find(
    config: &Config, repository: &str, branch: &str,
) -> Result<Vec<PullRequest>, ForgeError> {
    let (owner, repo) = github_repo(repository)?;
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|err| ForgeError::Internal(format!("building the HTTP client: {err}")))?;
    let head = format!("{owner}:{branch}");
    let mut results = Vec::new();
    for page in 1_u32.. {
        let url = format!(
            "{}/repos/{owner}/{repo}/pulls?state=all&head={head}&per_page={PER_PAGE}&page={page}",
            config.api_base
        );
        let rows = fetch_page(&client, config, &url)?;
        let exhausted = rows.len() < PER_PAGE;
        results.extend(rows.into_iter().filter(|row| row.head.label == head).map(wire));
        if exhausted {
            break;
        }
    }
    Ok(results)
}

const PER_PAGE: usize = 100;

/// Split `https://github.com/owner/repo[.git]` (scheme optional) into
/// its REST coordinates. GitHub is the only v1 forge (D10).
fn github_repo(repository: &str) -> Result<(String, String), ForgeError> {
    let rest = repository.strip_prefix("https://").unwrap_or(repository);
    let mut segments = rest.split('/');
    let host = segments.next().unwrap_or_default();
    if host != "github.com" {
        return Err(ForgeError::InvalidRequest(format!(
            "`{repository}` is not a GitHub repository — GitHub is the only v1 forge"
        )));
    }
    let owner = segments.next().unwrap_or_default();
    let repo = segments.next().unwrap_or_default().trim_end_matches(".git");
    if owner.is_empty() || repo.is_empty() || segments.next().is_some() {
        return Err(ForgeError::InvalidRequest(format!(
            "`{repository}` is not an `owner/repo` GitHub repository reference"
        )));
    }
    Ok((owner.to_string(), repo.to_string()))
}

fn fetch_page(
    client: &reqwest::blocking::Client, config: &Config, url: &str,
) -> Result<Vec<Row>, ForgeError> {
    let mut request = client
        .get(url)
        .header("Accept", "application/vnd.github+json")
        .header("User-Agent", "emery");
    if let Some(token) = &config.token {
        request = request.bearer_auth(token);
    }
    let response =
        request.send().map_err(|err| ForgeError::Transport(format!("GET {url}: {err}")))?;
    let status = response.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(ForgeError::Auth(format!("GET {url} returned {status}")));
    }
    if !status.is_success() {
        return Err(ForgeError::Transport(format!("GET {url} returned {status}")));
    }
    let body =
        response.text().map_err(|err| ForgeError::Transport(format!("reading {url}: {err}")))?;
    serde_json::from_str(&body)
        .map_err(|err| ForgeError::Transport(format!("decoding {url}: {err}")))
}

/// The GitHub REST subset one pull-request row carries.
#[derive(serde::Deserialize)]
struct Row {
    html_url: String,
    body: Option<String>,
    state: String,
    merged_at: Option<String>,
    merge_commit_sha: Option<String>,
    base: Ref,
    head: Head,
}

#[derive(serde::Deserialize)]
struct Ref {
    #[serde(rename = "ref")]
    name: String,
}

/// `label` is `owner:branch` — the same-repository head check (a
/// fork's head never matches, D10).
#[derive(serde::Deserialize)]
struct Head {
    label: String,
}

fn wire(row: Row) -> PullRequest {
    let merged = row.merged_at.is_some();
    let state = if merged {
        PrState::Merged
    } else if row.state == "open" {
        PrState::Open
    } else {
        PrState::Closed
    };
    PullRequest {
        url: row.html_url,
        body: row.body.unwrap_or_default(),
        state,
        base: row.base.name,
        merged_at: row.merged_at,
        merge_commit: merged
            .then(|| row.merge_commit_sha.unwrap_or_default())
            .filter(|sha| !sha.is_empty()),
    }
}

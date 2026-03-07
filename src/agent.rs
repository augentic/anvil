use std::path::Path;

use anyhow::{Context, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Claude,
    DryRun,
}

impl Backend {
    fn from_env() -> Self {
        let val = std::env::var("ALC_AGENT_BACKEND")
            .or_else(|_| std::env::var("OPSX_AGENT_BACKEND"))
            .unwrap_or_else(|_| String::from("claude"));
        match val.to_ascii_lowercase().as_str() {
            "dry-run" | "dry_run" | "dryrun" => Self::DryRun,
            _ => Self::Claude,
        }
    }
}

/// Invoke the configured agent backend for a command in a repo.
///
/// Set `ALC_AGENT_BACKEND=dry-run` to print commands without executing.
pub async fn invoke(command: &str, repo_dir: &Path) -> Result<bool> {
    let backend = Backend::from_env();
    tracing::info!(
        command,
        dir = %repo_dir.display(),
        ?backend,
        "invoking agent"
    );

    match backend {
        Backend::DryRun => {
            tracing::info!("dry-run backend selected; skipping agent execution");
            Ok(true)
        }
        Backend::Claude => {
            let status = tokio::process::Command::new("claude")
                .args(["--message", command, "--yes"])
                .arg("--directory")
                .arg(repo_dir)
                .status()
                .await
                .context("spawning claude CLI")?;

            if status.success() {
                tracing::info!("agent completed successfully");
            } else {
                tracing::warn!(code = ?status.code(), "agent exited with error");
            }
            Ok(status.success())
        }
    }
}

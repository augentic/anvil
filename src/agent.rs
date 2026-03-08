use std::path::Path;
use std::time::Duration;

use anyhow::{Context, Result, bail};

const DEFAULT_AGENT_TIMEOUT_SECS: u64 = 600;

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

fn timeout_from_env() -> Duration {
    let secs = std::env::var("ALC_AGENT_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(DEFAULT_AGENT_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

/// Invoke the configured agent backend for a command in a repo.
///
/// Set `ALC_AGENT_BACKEND=dry-run` to print commands without executing.
/// Set `ALC_AGENT_TIMEOUT_SECS` to override the default 600s timeout.
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
            let timeout = timeout_from_env();
            let child = tokio::process::Command::new("claude")
                .args(["--message", command, "--yes"])
                .arg("--directory")
                .arg(repo_dir)
                .stderr(std::process::Stdio::piped())
                .spawn()
                .context("spawning claude CLI — is it installed and on PATH?")?;

            let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
                Ok(out) => out.context("waiting for claude CLI")?,
                Err(_) => {
                    bail!(
                        "agent timed out after {}s (set ALC_AGENT_TIMEOUT_SECS to increase)",
                        timeout.as_secs()
                    );
                }
            };

            if output.status.success() {
                tracing::info!("agent completed successfully");
                Ok(true)
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                let code = output.status.code();
                tracing::warn!(code = ?code, %stderr, "agent exited with error");
                Ok(false)
            }
        }
    }
}

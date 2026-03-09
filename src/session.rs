use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::engine::opsx::OpsxEngine;

/// Runtime context shared across all commands. Constructed once at startup.
pub struct Session {
    pub workspace: PathBuf,
    pub engine: OpsxEngine,
    pub concurrency: usize,
    github: Option<octocrab::Octocrab>,
}

impl Session {
    pub fn new(workspace: PathBuf, concurrency: usize) -> Self {
        let github = std::env::var("GITHUB_TOKEN").ok().and_then(|token| {
            octocrab::Octocrab::builder()
                .personal_token(token)
                .build()
                .ok()
        });
        Self {
            workspace,
            engine: OpsxEngine,
            concurrency,
            github,
        }
    }

    pub fn github(&self) -> Result<&octocrab::Octocrab> {
        self.github
            .as_ref()
            .context("GITHUB_TOKEN env var required for GitHub API access")
    }
}

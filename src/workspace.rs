use std::path::PathBuf;

use anyhow::{Context, Result, bail};

/// Walk upward from the current directory to find the workspace root,
/// identified by the presence of `registry.toml`.
pub fn find_root() -> Result<PathBuf> {
    let start = std::env::current_dir().context("cannot read current directory")?;
    let mut dir = start.clone();
    loop {
        if dir.join("registry.toml").exists() {
            return Ok(dir);
        }
        if !dir.pop() {
            bail!(
                "could not find registry.toml in any parent of '{}'\n  \
                 hint: run `alc init` to create one, or cd into a directory that contains registry.toml",
                start.display()
            );
        }
    }
}

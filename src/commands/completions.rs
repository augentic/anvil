//! `specify completions` -- generate shell completions.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};
use clap::CommandFactory;
use clap_complete::generate;

use crate::cli::{ShellChoice, Specify};

/// Run the completions command.
///
/// # Errors
///
/// Returns an error if the output file cannot be created.
pub fn run(shell: ShellChoice, output: Option<&Path>) -> Result<()> {
    let mut cmd = Specify::command();
    let shell_type: clap_complete::Shell = shell.into();

    match output {
        Some(path) => {
            let mut file = std::fs::File::create(path)
                .with_context(|| format!("creating {}", path.display()))?;
            generate(shell_type, &mut cmd, "specify", &mut file);
            file.flush()?;
            println!("  Wrote completions to {}", path.display());
        }
        None => {
            generate(shell_type, &mut cmd, "specify", &mut std::io::stdout());
        }
    }

    Ok(())
}

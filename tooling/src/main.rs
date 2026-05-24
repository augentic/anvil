use std::process;

use clap::{Parser, Subcommand};
use tooling::check;
use tooling::context::Context;
use tooling::error::ToolingError;
use tooling::exit::{exit_from_result, Exit};

#[derive(Debug, Parser)]
#[command(name = "tooling", about = "Framework developer tooling for augentic/specify")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Run framework consistency checks over the repo root.
    Check,
    /// Generate or verify generated documentation.
    Docgen {
        #[command(subcommand)]
        target: DocgenTarget,
    },
}

#[derive(Debug, Subcommand)]
enum DocgenTarget {
    /// Regenerate CLI output-shape docs from specify-cli fixtures.
    Envelopes {
        /// Exit 2 when generated output would drift.
        #[arg(long)]
        check: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Check => run_check(),
        Command::Docgen {
            target: DocgenTarget::Envelopes { check },
        } => run_docgen_envelopes(check),
    };
    process::exit(i32::from(code.code()));
}

fn run_check() -> Exit {
    let result = (|| -> Result<Vec<tooling::finding::Finding>, ToolingError> {
        let ctx = Context::discover()?;
        Ok(check::run(&ctx))
    })();

    match &result {
        Ok(findings) if findings.is_empty() => {}
        Ok(findings) => {
            for finding in findings {
                eprintln!("FAIL: {}: {}", finding.rule_id, finding.message);
                if let Some(location) = &finding.location {
                    eprintln!("  at {}:{}", location.path.display(), location.line);
                }
            }
        }
        Err(error) => eprintln!("error: {error}"),
    }

    match result {
        Ok(findings) => exit_from_result(Ok(()), findings.len()),
        Err(error) => exit_from_result(Err(error), 0),
    }
}

fn run_docgen_envelopes(check: bool) -> Exit {
    let result = (|| -> Result<Exit, ToolingError> {
        let ctx = Context::discover()?;
        tooling::docgen::run_envelopes(ctx.framework_root(), &ctx.specify_cli_dir(), check)
    })();

    match &result {
        Ok(_) => {}
        Err(error) => eprintln!("error: {error}"),
    }

    match result {
        Ok(exit) => exit,
        Err(ToolingError::Validation(_)) => Exit::ValidationFailed,
        Err(ToolingError::Infrastructure(_)) => Exit::GenericFailure,
    }
}

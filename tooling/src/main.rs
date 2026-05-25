use std::path::Path;
use std::process;

use clap::{Parser, Subcommand};
use tooling::check;
use tooling::context::Context;
use tooling::error::ToolingError;
use tooling::exit::{exit_from_result, Exit};
use tooling::finding::{Finding, Location};

#[derive(Debug, Parser)]
#[command(
    name = "tooling",
    about = "Framework developer tooling for augentic/specify",
    version,
    after_help = "Common entry points:\n  make check                       # runs `tooling check` in release mode\n  make ci                          # check + tests + envelope drift\n  cargo docgen-envelopes --verify  # CI drift check for cli-output-shapes.md"
)]
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
        /// Exit 2 when generated output would drift instead of rewriting the doc.
        #[arg(long, alias = "check")]
        verify: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Check => run_check(),
        Command::Docgen {
            target: DocgenTarget::Envelopes { verify },
        } => run_docgen_envelopes(verify),
    };
    process::exit(i32::from(code.code()));
}

fn run_check() -> Exit {
    let result = (|| -> Result<(std::path::PathBuf, Vec<Finding>), ToolingError> {
        let ctx = Context::discover()?;
        let framework_root = ctx.framework_root().to_path_buf();
        Ok((framework_root, check::run(&ctx)))
    })();

    match &result {
        Ok((_, findings)) if findings.is_empty() => {
            println!("All checks passed.");
        }
        Ok((framework_root, findings)) => {
            for finding in findings {
                eprintln!("FAIL: {}: {}", finding.rule_id, finding.message);
                if let Some(location) = &finding.location {
                    eprintln!("  at {}", format_location(framework_root, location));
                }
            }
            eprintln!("{} check failure(s).", findings.len());
        }
        Err(error) => eprintln!("error: {error}"),
    }

    match result {
        Ok((_, findings)) => exit_from_result(Ok(()), findings.len()),
        Err(error) => exit_from_result(Err(error), 0),
    }
}

fn format_location(framework_root: &Path, location: &Location) -> String {
    let path = location
        .path
        .strip_prefix(framework_root)
        .unwrap_or(&location.path)
        .display()
        .to_string()
        .replace('\\', "/");

    match location.column {
        Some(column) => format!("{path}:{}:{column}", location.line),
        None => format!("{path}:{}", location.line),
    }
}

fn run_docgen_envelopes(verify: bool) -> Exit {
    let result = (|| -> Result<Exit, ToolingError> {
        let ctx = Context::discover()?;
        tooling::docgen::run_envelopes(ctx.framework_root(), &ctx.specify_cli_dir(), verify)
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

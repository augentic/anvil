//! Entry point for the `anvil` CLI.

use anvil::cli::{Anvil, Command};
use clap::Parser;
use tracing_subscriber::EnvFilter;

fn main() {
    let cli = Anvil::parse();

    init_tracing(cli.verbose, cli.quiet);

    if let Err(err) = run(cli.command) {
        eprintln!("error: {err}");
        for cause in err.chain().skip(1) {
            eprintln!("  caused by: {cause}");
        }
        std::process::exit(1);
    }
}

fn run(command: Command) -> anyhow::Result<()> {
    match command {
        Command::Init { schema, context } => anvil::commands::init::run(schema, context),
        Command::Update {
            project,
            repo,
            git_ref,
        } => anvil::commands::update::run(project, &repo, &git_ref),
        Command::Validate => anvil::commands::validate::run(),
        Command::Schemas => anvil::commands::schemas::run(),
        Command::Completions { shell, output } => {
            anvil::commands::completions::run(shell, output.as_deref())
        }
    }
}

fn init_tracing(verbose: u8, quiet: bool) {
    let filter = if quiet {
        "error"
    } else {
        match verbose {
            0 => "warn",
            1 => "info,anvil=debug",
            2.. => "trace",
        }
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(filter)),
        )
        .with_target(false)
        .without_time()
        .init();
}

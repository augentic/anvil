//! Entry point for the `alc` CLI.

use clap::Parser;
use tracing_subscriber::EnvFilter;

use alc::cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

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
        Command::Init {
            schema,
            context,
            force,
        } => alc::commands::init::run(schema, context, force),

        Command::Update {
            project,
            repo,
            git_ref,
        } => alc::commands::update::run(project, &repo, &git_ref),

        Command::New { name } => alc::commands::new::run(&name),

        Command::Validate => alc::commands::validate::run(),

        Command::Schemas => alc::commands::schemas::run(),

        Command::Completions { shell, output } => {
            alc::commands::completions::run(shell, output.as_deref())
        }
    }
}

fn init_tracing(verbose: u8, quiet: bool) {
    let filter = if quiet {
        "error"
    } else {
        match verbose {
            0 => "warn",
            1 => "info,alc=debug",
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

//! Entry point for the `specify` CLI.

use clap::Parser;
use specify::cli::{Command, Specify};
use tracing_subscriber::EnvFilter;

fn main() {
    let cli = Specify::parse();

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
        Command::Init { schema } => specify::commands::init::run(schema),
        Command::New { name, json } => specify::commands::new::run(&name, json),
        Command::Status { change, json } => specify::commands::status::run(change.as_deref(), json),
        Command::Instructions {
            artifact,
            change,
            json,
        } => specify::commands::instructions::run(&artifact, change.as_deref(), json),
        Command::List { json } => specify::commands::list::run(json),
        Command::Archive { change, json } => specify::commands::archive::run(&change, json),
        Command::Update {
            project,
            repo,
            git_ref,
        } => specify::commands::update::run(project, &repo, &git_ref),
        Command::Validate => specify::commands::validate::run(),
        Command::Schemas => specify::commands::schemas::run(),
        Command::Completions { shell, output } => {
            specify::commands::completions::run(shell, output.as_deref())
        }
    }
}

fn init_tracing(verbose: u8, quiet: bool) {
    let filter = if quiet {
        "error"
    } else {
        match verbose {
            0 => "warn",
            1 => "info,specify=debug",
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

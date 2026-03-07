use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lc", about = "Multi-repo orchestration for spec-driven development")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Generate planning artefacts for a new change in the hub repo
    Propose {
        /// Change name (e.g., r9k-http)
        change: String,
        /// Human description of the initiative
        #[arg(long)]
        description: String,
        /// Preview the prompt without invoking the agent
        #[arg(long)]
        dry_run: bool,
    },
    /// Distribute change to target repos, open draft PRs
    FanOut {
        /// Change name (e.g., r9k-http)
        change: String,
        /// Preview what would happen without executing
        #[arg(long)]
        dry_run: bool,
    },
    /// Invoke agent per target in dependency order
    Apply {
        /// Change name
        change: String,
        /// Apply only a specific target
        #[arg(long)]
        target: Option<String>,
        /// Preview what would happen without invoking agents
        #[arg(long)]
        dry_run: bool,
    },
    /// Show pipeline status for all targets
    Status {
        /// Change name
        change: String,
    },
    /// Archive completed change in the hub
    Archive {
        /// Change name
        change: String,
        /// Preview what would happen without archiving
        #[arg(long)]
        dry_run: bool,
    },
    /// Synchronize PR state into status.toml
    Sync {
        /// Change name
        change: String,
        /// Mark draft PRs as ready for review when implemented
        #[arg(long)]
        mark_ready: bool,
    },
    /// Query the service registry
    Registry {
        #[command(subcommand)]
        action: RegistryAction,
    },
}

#[derive(Subcommand)]
pub enum RegistryAction {
    /// List all services
    List,
    /// Query services by domain or capability
    Query {
        /// Filter by domain
        #[arg(long)]
        domain: Option<String>,
        /// Filter by capability
        #[arg(long)]
        cap: Option<String>,
    },
}

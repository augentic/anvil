use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "alc",
    about = "Multi-repo orchestration for spec-driven development",
    after_help = "\
Typical workflow:
  alc propose <change> -d \"description\"   Generate planning artefacts
  alc apply <change>                       Distribute specs, open draft PRs, invoke agent
  alc archive <change>                     Sync PR state, auto-archive when all merged"
)]
pub struct Cli {
    /// Increase log verbosity to debug level
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,
    /// Decrease log verbosity to warnings only
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,
    /// Max concurrent repo operations
    #[arg(long, short = 'j', global = true, default_value = "4")]
    pub concurrency: usize,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Initialise a new hub workspace with registry.toml and directory layout
    Init,
    /// Generate planning artefacts for a new change in the hub repo
    Propose {
        /// Change name (e.g., r9k-http)
        change: String,
        /// Human description of the initiative
        #[arg(long, short = 'd')]
        description: String,
        /// Preview the prompt without invoking the agent
        #[arg(long)]
        dry_run: bool,
    },
    /// Distribute specs, open draft PRs, and invoke agent per repo group in dependency order
    Apply {
        /// Change name
        change: String,
        /// Apply only the repo group containing this target
        #[arg(long)]
        target: Option<String>,
        /// Preview what would happen without invoking agents
        #[arg(long)]
        dry_run: bool,
        /// Continue executing independent groups when one fails
        #[arg(long)]
        continue_on_failure: bool,
    },
    /// Show pipeline status for all targets
    Status {
        /// Change name
        change: String,
    },
    /// Synchronize PR state from GitHub; auto-archive when all targets are merged
    Archive {
        /// Change name
        change: String,
        /// Mark draft PRs as ready for review when implemented
        #[arg(long)]
        mark_ready: bool,
    },
    /// Validate pipeline, registry, and status consistency for a change
    Validate {
        /// Change name
        change: String,
    },
    /// List existing changes in the hub
    List,
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

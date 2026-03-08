use clap::{Parser, Subcommand, ValueEnum};

/// Supported spec engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, ValueEnum)]
pub enum EngineKind {
    #[default]
    Opsx,
}

#[derive(Parser)]
#[command(
    name = "alc",
    about = "Multi-repo orchestration for spec-driven development",
    after_help = "\
Typical workflow:
  alc propose <change> -d \"description\"   Generate planning artefacts
  alc fan-out <change>                     Distribute to target repos, open draft PRs
  alc apply <change>                       Invoke agent per repo to implement
  alc sync <change>                        Sync PR state from GitHub
  alc archive <change>                     Archive after all PRs merged"
)]
pub struct Cli {
    /// Increase log verbosity to debug level
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,
    /// Decrease log verbosity to warnings only
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,
    /// Max concurrent repo operations (fan-out, apply)
    #[arg(long, short = 'j', global = true, default_value = "4")]
    pub concurrency: usize,
    /// Spec engine to use
    #[arg(long, global = true, default_value = "opsx", value_enum)]
    pub engine: EngineKind,
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
    /// Distribute change to target repos, open draft PRs
    FanOut {
        /// Change name (e.g., r9k-http)
        change: String,
        /// Preview what would happen without executing
        #[arg(long)]
        dry_run: bool,
    },
    /// Invoke agent per repo group in dependency order
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

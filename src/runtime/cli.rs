//! The native provisioning grammar — the closed verb set the
//! capability fence keeps native, plus the acknowledged workspace
//! residue and the hidden `lint framework` dev tool.
//!
//! This is deliberately *not* the operational grammar: workflow argv
//! never parses natively. The binary's first-token triage
//! ([`crate::runtime::run`]) admits only the verbs below into this
//! parser; everything else forwards unparsed to the workflow guest,
//! whose grammar (`specify_dispatch::cli::Cli`) also carries these
//! verbs so `--help` and completions stay whole — execution is
//! native-only, help is guest-served. The per-verb argument structs
//! ([`InitArgs`], [`UpgradeArgs`], the action enums) are shared with
//! that grammar, so both sides parse byte-identical argv.

use clap::{Parser, Subcommand};
pub use specify_dispatch::cli::ChannelArg;
use specify_dispatch::cli::{ABOUT, InitArgs, UpgradeArgs};
use specify_dispatch::commands::adapters::cli::AdaptersAction;
use specify_dispatch::commands::lint::cli::LintAction;
use specify_dispatch::commands::plugins::cli::PluginsAction;
use specify_dispatch::commands::workspace::cli::WorkspaceAction;
pub use specify_dispatch::output::Format;

/// First tokens (after the global flags) the native grammar owns; any
/// other leading token forwards unparsed to the workflow guest.
pub const NATIVE_VERBS: &[&str] = &["init", "adapters", "upgrade", "plugins", "workspace", "lint"];

/// Parsed native `specify` invocation: a provisioning (or residue)
/// subcommand plus the global flags the native handlers read.
#[derive(Debug, Parser)]
#[command(name = "specify", version, about = ABOUT)]
pub struct Cli {
    /// The verb to run.
    #[command(subcommand)]
    pub command: Commands,

    /// Output format. `text` by default; pass `--format json` (or set
    /// `SPECIFY_FORMAT=json`) for structured envelopes when shelling
    /// out from skills.
    #[arg(long, env = "SPECIFY_FORMAT", default_value = "text", global = true)]
    pub format: Format,

    /// Plan root override: the directory holding the governing
    /// `plan.yaml` when it is not the project root. Native-only:
    /// forwarded verbs anchor plan artifacts at the working directory
    /// and refuse any other plan root.
    #[arg(long, env = "SPECIFY_PLAN_DIR", value_name = "PATH", global = true)]
    pub plan_dir: Option<std::path::PathBuf>,
}

/// The closed native verb set.
///
/// `init`, `adapters sync`, `upgrade`, and `plugins` are native
/// because they cannot run in-guest by construction (network pulls,
/// global-store writes, binary self-replacement, dev-machine probing).
/// The `workspace` family is the one acknowledged residue — a
/// subprocess-`git` surface that flips guest-side when the host git
/// capability lands. `lint framework` is framework-repo CI tooling,
/// hidden from operator help.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize .specify/ in a project.
    ///
    /// Pass `<adapter>` (first-party shorthand, local path, or URL) for
    /// a regular project, or `--workspace` for a registry-only
    /// workspace. The two are mutually exclusive — clap enforces the
    /// `<adapter>` xor `--workspace` shape and exits `2` with its
    /// standard parse-error diagnostic when the invariant is violated.
    Init(InitArgs),

    /// Global adapter-store provisioning. `sync` is the explicit
    /// hydration trigger: it hydrates every pinned identity the
    /// project declares (`project.yaml` plus `plan.yaml` source pins)
    /// into the global store, re-materializes the shared codex packs,
    /// regenerates the deployment manifest, and prints the resolved
    /// set. Native provisioning verb — never runs in the workflow
    /// guest.
    Adapters {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: AdaptersAction,
    },

    /// Self-update the `specify` binary across its install channel.
    ///
    /// Bootstrap verb: operates on the binary, not a project, so it
    /// never loads project config. `--channel auto` (the default)
    /// detects how the binary was installed (`cargo`, Homebrew, or a
    /// pre-built release archive); pass `--channel` to override. The
    /// target version is the latest GitHub release when reachable,
    /// otherwise a HEAD install for the `cargo` channel. `--dry-run`
    /// reports the detected channel, the target version, and the exact
    /// command(s) that would run without changing anything; applying
    /// requires `--yes` (the verb never prompts).
    Upgrade(UpgradeArgs),

    /// Inspect and invalidate the Cursor plugin cache.
    ///
    /// Bootstrap verb: operates on `$CURSOR_HOME/plugins/cache/<name>/`
    /// and the marketplace manifest, not a project, so it never loads
    /// project config. `doctor` reports per-plugin drift (read-only);
    /// `refresh` clears the marketplace-scoped cache after `--yes` and
    /// prints a restart instruction. The CLI never restarts Cursor.
    Plugins {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: PluginsAction,
    },

    /// Materialise and manage registry peers under `workspace/`.
    Workspace {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: WorkspaceAction,
    },

    /// Framework-repo CI lint (`make lint`). Dev tooling, not an
    /// operational verb — hidden from help on both sides of the seam.
    #[command(hide = true, subcommand_required = true)]
    Lint {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: LintAction,
    },
}

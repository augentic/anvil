//! Top-level clap derive surface for the `specify` binary.
//!
//! Owns the umbrella types ([`Cli`], [`Commands`], [`Format`]), the
//! shared [`parse`] entry point, and the re-exports of the per-verb
//! action enums. Custom field grammars (`SourceAssign`,
//! `BindingArg`, `KindAssign`) live in
//! `workflow::change::plan::handlers` — one type per grammar carrying
//! both the clap `FromStr` and the serde wire form.

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;

use crate::commands::adapters::cli::AdaptersAction;
use crate::commands::archive::cli::ArchiveAction;
use crate::commands::journal::cli::JournalAction;
use crate::commands::plan::cli::PlanAction;
use crate::commands::plugins::cli::PluginsAction;
use crate::commands::registry::cli::RegistryAction;
use crate::commands::slice::cli::SliceAction;
use crate::commands::source::cli::SourceAction;
use crate::commands::target::cli::TargetAction;
use crate::commands::workspace::cli::WorkspaceAction;
use crate::output::Exit;
pub use crate::output::Format;

/// The one-line `about` string shared by the full grammar here and the
/// native provisioning grammar in the `specify` binary, so `--help`
/// reads identically on both sides of the seam.
pub const ABOUT: &str = "Deterministic primitives for spec-driven development";

/// Parsed `specify` invocation: the subcommand plus the global flags.
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
    /// `plan.yaml` (plus `change.md` / `discovery.md`) when it is not
    /// the project root. Set by the `/spec:execute` routing layer (or
    /// `SPECIFY_PLAN_DIR`) to the initiating workspace root while
    /// phase verbs run inside a workspace slot, so slice-time plan
    /// reads and the merge `done` stamp resolve against the
    /// workspace's plan.
    #[arg(long, env = "SPECIFY_PLAN_DIR", value_name = "PATH", global = true)]
    pub plan_dir: Option<std::path::PathBuf>,
}

/// The full `specify` operational verb tree.
///
/// The provisioning families are included, so `--help` and shell
/// completions see one grammar wherever the parse runs. Provisioning
/// verbs are *executed* natively only: the guest router refuses them,
/// and the binary's first-token triage never forwards them.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize .specify/ in a project.
    ///
    /// Pass `<adapter>` (first-party shorthand, local path, or URL) for
    /// a regular project, or `--workspace` for a registry-only
    /// workspace. The two are mutually exclusive — clap enforces the
    /// conflict and exits `2` with its standard parse-error diagnostic.
    /// A missing `<adapter>` reaches the native elicitation layer:
    /// prompted on a TTY, the typed `init-adapter-required` (exit 2)
    /// everywhere else.
    Init(InitArgs),

    /// Global adapter-store provisioning. `sync` is the
    /// explicit hydration trigger: it hydrates every pinned identity
    /// the project declares (`project.yaml` plus `plan.yaml` source
    /// pins) into the global store and prints the resolved set. Native
    /// provisioning verb — never runs in the workflow guest.
    Adapters {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: AdaptersAction,
    },

    /// Source adapter operations (workflow contract). Source adapters
    /// provide `extract` + `survey` capabilities and resolve to a single
    /// `.wasm` component: the global store entry for pinned identities,
    /// the development release build for bare names.
    Source {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: SourceAction,
    },

    /// Target adapter operations (workflow contract). Target adapters
    /// provide `guidance` + `build` + `merge` capabilities and resolve to a
    /// single `.wasm` component: the global store entry for pinned
    /// identities, the development release build for bare names.
    Target {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: TargetAction,
    },

    /// Slice lifecycle operations — one `refine → build → merge` loop.
    Slice {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: SliceAction,
    },

    /// Slice-archive cache maintenance. The archived slice folders
    /// under `.specify/archive/` are a prunable convenience cache;
    /// `prune` reclaims disk by retention bound.
    Archive {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: ArchiveAction,
    },

    /// Executable plan operations — `plan.yaml` lifecycle.
    Plan {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: PlanAction,
    },

    /// Workflow journal at `.specify/journal.jsonl`. `emit` is a
    /// guarded front door onto the closed §Observability event
    /// taxonomy — it appends one well-formed line, minting no event
    /// kinds of its own.
    Journal {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: JournalAction,
    },

    /// Platform registry at `registry.yaml` (repo root)
    Registry {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: RegistryAction,
    },

    /// Materialise and manage registry peers under `workspace/`.
    Workspace {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: WorkspaceAction,
    },

    /// Print a shell-completion script for `<shell>` to stdout.
    ///
    /// Pipe into your shell's completion directory (e.g.
    /// `specify completions zsh > ~/.zsh/_specify`). Generated via
    /// `clap_complete`; the output tracks the live clap surface so
    /// every new verb is auto-discovered.
    Completions {
        /// Target shell — one of `bash`, `elvish`, `fish`, `powershell`, `zsh`.
        shell: Shell,
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
}

/// Flag surface for `specify init`.
///
/// One shared struct so the full grammar here and the native
/// provisioning grammar in the `specify` binary parse byte-identical
/// argv (the binary executes the verb; this grammar carries it for
/// `--help` and completions, with the guest refusing everything but
/// the hidden `--scaffold-only` leg).
///
/// Doubles as the argv mirror of the scaffold leg's wire input
/// (`workflow::init::handlers::ScaffoldInput`): the routing arms pass
/// it whole and the extra provisioning flags (`upgrade`,
/// `scaffold-only`) are ignored keys on the wire.
#[derive(Debug, Args, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct InitArgs {
    /// Adapter identifier. A package reference
    /// (`specify:omnia@1.0.0`) or the first-party shorthand
    /// (`omnia@1.0.0` — package-reference sugar installing the
    /// published component; bare `omnia` — the development shorthand
    /// resolving the sibling/in-repo release build). Also accepts a
    /// local `.wasm` component path (`./omnia.wasm`, `file://…`).
    /// GitHub URLs are refused (`adapter-github-uri-unsupported`).
    /// Required unless `--workspace` or `--upgrade` is set —
    /// left open at the clap surface so the native elicitation layer
    /// can prompt on a TTY and fail with the typed
    /// `init-adapter-required` (exit 2) elsewhere. Mutually
    /// exclusive with `--workspace`.
    #[arg(conflicts_with = "workspace")]
    pub adapter: Option<String>,
    /// Project name (defaults to the project directory name)
    #[arg(long)]
    pub name: Option<String>,
    /// Project description (tech stack, architecture, testing)
    #[arg(long)]
    pub description: Option<String>,
    /// Scaffold a registry-only workspace instead of a regular
    /// project. Refuses to run when `.specify/` already exists.
    #[arg(long)]
    pub workspace: bool,
    /// Comma-separated target platforms (e.g. core,ios,android).
    /// Required when the target adapter declares platforms as mandatory.
    /// Run `specify init <adapter>` without --platforms to see the
    /// target's allowed and default sets.
    #[arg(long, conflicts_with = "workspace")]
    pub platforms: Option<String>,
    /// Re-entry version bump: over an already-populated `.specify/`,
    /// rewrite `project.yaml.specify` to this binary's
    /// version (preserving every other field) and regenerate
    /// `AGENTS.md` only when absent — scaffolding nothing else and
    /// never re-fetching the adapter cache. A project already at the
    /// running version is a no-op. Mutually exclusive with every
    /// other `init` argument except `--platforms`.
    #[arg(long, conflicts_with_all = ["adapter", "workspace", "name", "description"])]
    pub upgrade: bool,
    /// Run only the project-scoped scaffold leg (`.specify/`,
    /// `project.yaml`, workspace `registry.yaml`) — no hydration,
    /// no deployment-manifest generation, no `AGENTS.md` context
    /// generation. The guest-invocable half of `init`; the
    /// provisioning half awaits its in-guest implementation.
    /// Hidden — not an operator-facing verb.
    #[arg(long, hide = true, conflicts_with = "upgrade")]
    pub scaffold_only: bool,
}

/// Flag surface for `specify upgrade`.
#[derive(Debug, Clone, Copy, Args)]
pub struct UpgradeArgs {
    /// Install channel to upgrade. `auto` detects it from the
    /// running binary's path; `cargo` / `brew` / `binary` force a
    /// specific strategy.
    #[arg(long, value_enum, default_value = "auto")]
    pub channel: ChannelArg,
    /// Apply the upgrade. Required to mutate the binary; the verb
    /// never prompts interactively.
    #[arg(long)]
    pub yes: bool,
    /// Report the detected channel, target version, and the
    /// command(s) that would run without changing anything.
    #[arg(long)]
    pub dry_run: bool,
}

/// `specify upgrade --channel` value.
///
/// `Auto` resolves to `workflow::upgrade::InstallChannel::detect`
/// at the handler boundary (a `native`-feature module this wasm-clean
/// crate never links); the other variants force the matching channel.
#[derive(Copy, Clone, Debug, ValueEnum, PartialEq, Eq)]
pub enum ChannelArg {
    /// Detect the install channel from the running binary's path.
    Auto,
    /// Force the `cargo install --git` strategy.
    Cargo,
    /// Force the `brew upgrade` strategy.
    Brew,
    /// Force the release-archive (binary) strategy.
    Binary,
}

/// Parse argv through the shared grammar.
///
/// This is the exact clap tree the operator's `specify` invocation
/// forwards into, so `--help`, error text, and usage exits match on
/// every shim. A host may supply `argv[0]` as something other than
/// the binary name (the wasm deployment's guest id), so it is
/// replaced with `specify` before parsing — clap renders `argv[0]`
/// into every usage line, and the operator typed `specify`.
///
/// The parse is `try_parse` on purpose: `clap::Error::exit_code()`
/// travels back as an [`Exit`] the shim hands to its exit surface.
/// `parse()`'s internal `process::exit` would land on the p2 exit in
/// a wasm guest, which carries only success/failure and would
/// collapse clap's usage-error code `2` to `1`.
///
/// # Errors
///
/// On a parse failure (or `--help` / `--version`) clap's rendering is
/// written to the conventional sink (stdout for help/version, stderr
/// for usage errors) and the matching process exit code comes back as
/// an [`Exit`] for passthrough.
pub fn parse(argv: impl IntoIterator<Item = String>) -> Result<Cli, Exit> {
    let forwarded = std::iter::once("specify".to_string()).chain(argv.into_iter().skip(1));
    Cli::try_parse_from(forwarded).map_err(|err| {
        // clap's own printer keeps help on stdout and errors on
        // stderr; a sink failure leaves nothing better to do than
        // carry the exit code through.
        drop(err.print());
        Exit::Code(u8::try_from(err.exit_code()).unwrap_or(1))
    })
}

/// Print the shell-completion script for `shell` to stdout — pure
/// stdout from the shared clap grammar, so the output is
/// byte-identical on every shim.
pub fn completions(shell: Shell) -> Exit {
    let mut cmd = <Cli as clap::CommandFactory>::command();
    clap_complete::generate(shell, &mut cmd, "specify", &mut std::io::stdout());
    Exit::Success
}

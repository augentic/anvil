//! Top-level clap derive surface for the `specify` binary. Owns the
//! umbrella types ([`Cli`], [`Commands`], [`Format`], [`SourceArg`],
//! [`SliceSourceArg`]) and re-exports the per-verb action enums.

use std::str::FromStr;

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_complete::Shell;
use specify_model::evidence::ClaimKind;

use crate::commands::adapters::cli::AdaptersAction;
use crate::commands::archive::cli::ArchiveAction;
use crate::commands::journal::cli::JournalAction;
use crate::commands::plan::cli::PlanAction;
use crate::commands::plugins::cli::PluginsAction;
use crate::commands::registry::cli::RegistryAction;
use crate::commands::rules::cli::RulesAction;
use crate::commands::slice::cli::SliceAction;
use crate::commands::source::cli::SourceAction;
use crate::commands::target::cli::TargetAction;
use crate::commands::workspace::cli::WorkspaceAction;
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

    /// Rules resolution operations. Read-only: no
    /// `.specify/` writes, no journal events. Today the only verb is
    /// `export`, which streams a `ResolvedRules` JSON envelope built
    /// from the shared / source / target codex overlay tree.
    Rules {
        /// Nested action for this verb family.
        #[command(subcommand)]
        action: RulesAction,
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
#[derive(Debug, Args)]
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
    /// generation. The guest-invocable half of `init`: the
    /// provisioning front calls it through the host form after
    /// hydration. Hidden — not an operator-facing verb.
    #[arg(long, hide = true, conflicts_with = "upgrade")]
    pub scaffold_only: bool,
}

/// Flag surface for `specify upgrade` — shared with the native
/// provisioning grammar exactly like [`InitArgs`].
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
/// `Auto` resolves to `specify_workflow_lib::upgrade::InstallChannel::detect`
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

/// Typed `--source <key>=<adapter>:<binding>` CLI value (top-level
/// plan source binding).
///
/// Wire grammar (locked):
///
/// - `--source <key>=<adapter>:<path>` — path-bound binding. The
///   adapter is the substring up to the first `:` after `=`; the
///   path is everything after that first `:` (URLs containing
///   `:` such as `git@github.com:org/foo.git` round-trip cleanly).
/// - `--source <key>=<adapter>:value:<literal>` — value-bound
///   binding. The `value:` sentinel after the adapter switches the
///   parser to literal mode; the literal payload is everything
///   after the second `:` and may contain anything (newlines,
///   colons, equals signs).
///
/// Materialises as [`specify_workflow_lib::change::SourceBinding`] under
/// the structured `{ adapter, path?, value? }` wire form. The legacy
/// bare-string `--source <key>=<path>` form was dropped — every
/// binding now carries an explicit adapter name.
///
/// The [`FromStr`] impl returns a `String` error on malformed input
/// so clap surfaces a standard usage diagnostic (exit code 2).
#[derive(Clone, Debug)]
pub struct SourceArg {
    /// Source key (left of `=`).
    pub(crate) key: String,
    /// Kebab-case source-adapter name (parsed out of the `<adapter>:…`
    /// prefix after `=`).
    pub(crate) adapter: String,
    /// Mutually exclusive with `value`. `Some(path)` for the
    /// `<adapter>:<path>` form.
    pub(crate) path: Option<String>,
    /// Mutually exclusive with `path`. `Some(literal)` for the
    /// `<adapter>:value:<literal>` form.
    pub(crate) value: Option<String>,
}

impl SourceArg {
    /// The desugared `plan create --intent <string>` binding —
    /// byte-identical to parsing `intent=intent:value:<string>`.
    #[must_use]
    pub(crate) fn intent(value: String) -> Self {
        Self {
            key: "intent".to_string(),
            adapter: "intent".to_string(),
            path: None,
            value: Some(value),
        }
    }
}

impl FromStr for SourceArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (key, rest) = s.split_once('=').ok_or_else(|| {
            format!(
                "--source must be <key>=<adapter>:<path> or <key>=<adapter>:value:<literal>, got \
                 `{s}`"
            )
        })?;
        if key.is_empty() {
            return Err(format!("--source key must be non-empty, got `{s}`"));
        }
        let (adapter, body) = rest.split_once(':').ok_or_else(|| {
            format!(
                "--source value must be <adapter>:<path> or <adapter>:value:<literal>, got \
                 `{rest}` for key `{key}`"
            )
        })?;
        if adapter.is_empty() {
            return Err(format!("--source adapter must be non-empty, got `{s}`"));
        }
        if body.is_empty() {
            return Err(format!(
                "--source binding (path or `value:<literal>`) must be non-empty, got `{s}`"
            ));
        }
        let (path, value) = if let Some(literal) = body.strip_prefix("value:") {
            if literal.is_empty() {
                return Err(format!(
                    "--source value-literal must be non-empty after `value:`, got `{s}`"
                ));
            }
            (None, Some(literal.to_string()))
        } else {
            (Some(body.to_string()), None)
        };
        Ok(Self {
            key: key.to_string(),
            adapter: adapter.to_string(),
            path,
            value,
        })
    }
}

/// Typed value for the per-slice `--sources` / `--add-source` /
/// `--remove-source` flags.
///
/// Wire forms (workflow §`Slice.sources`):
///
/// - `<key>=<lead>` — structured binding; both sides are
///   non-empty kebab identifiers. Materialises via
///   [`specify_workflow_lib::change::SliceSourceBinding::structured`].
/// - `<key>` — bare-string shorthand; sugar for
///   `{ key: <key>, lead: <slice.name> }`. Materialises via
///   [`specify_workflow_lib::change::SliceSourceBinding::bare`].
///
/// Malformed inputs (empty key, empty lead, dangling `=`, more
/// than one `=`) produce a `FromStr` error that clap surfaces as a
/// standard usage diagnostic (exit code 2 via `Error::Argument` at
/// the handler boundary).
#[derive(Clone, Debug)]
pub struct SliceSourceArg {
    pub(crate) key: String,
    /// `None` when the operator wrote the bare-string shorthand;
    /// `Some(lead)` otherwise. The handler downconverts to the
    /// bare wire form when `lead == slice.name` so the on-disk
    /// `plan.yaml` stays minimal.
    pub(crate) lead: Option<String>,
}

/// Typed value for the per-slice `--authority-override <kind>=<key>`
/// flag on `specify plan add` (where the slice context is implicit
/// from the command's positional `name`).
///
/// Wire form is `<claim-kind>=<source>`; both sides must be
/// non-empty and kebab-case (`source` is validated at the
/// `specify slice validate` stage via the orphan-key check).
/// `claim-kind` is parsed at the CLI boundary against the closed
/// [`ClaimKind`] enum so misspellings fail before any plan mutation
/// runs (clap exits 2 with its standard usage diagnostic).
#[derive(Clone, Debug)]
pub struct AuthorityOverrideKindAssign {
    pub(crate) kind: ClaimKind,
    pub(crate) source: String,
}

impl FromStr for AuthorityOverrideKindAssign {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (raw_kind, source) = s
            .split_once('=')
            .ok_or_else(|| format!("--authority-override must be <kind>=<source>, got `{s}`"))?;
        if raw_kind.is_empty() || source.is_empty() {
            return Err(format!(
                "--authority-override kind and source must both be non-empty, got `{s}`"
            ));
        }
        if source.contains('=') {
            return Err(format!(
                "--authority-override value `{s}` must contain exactly one `=` separator between \
                 kind and source"
            ));
        }
        let kind: ClaimKind = raw_kind.parse()?;
        Ok(Self {
            kind,
            source: source.to_string(),
        })
    }
}

impl FromStr for SliceSourceArg {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err("--sources value must be non-empty".to_string());
        }
        if let Some((k, v)) = s.split_once('=') {
            if v.contains('=') {
                return Err(format!(
                    "--sources value `{s}` must be <key>=<lead> with at most one `=`"
                ));
            }
            if k.is_empty() || v.is_empty() {
                return Err(format!("--sources key and lead must both be non-empty, got `{s}`"));
            }
            Ok(Self {
                key: k.to_string(),
                lead: Some(v.to_string()),
            })
        } else {
            Ok(Self {
                key: s.to_string(),
                lead: None,
            })
        }
    }
}

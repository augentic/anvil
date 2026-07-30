//! Typed command grammar, conversions, and Emery projection policy.

use clap::Args;
use omnia_guest::Model;
use omnia_guest::api::Provider;
use omnia_guest::api::command::{
    BuildError, CommandResponse, Completions, Namespace, Outcome, Projector, Router, RouterBuilder,
    run,
};
use omnia_guest::api::invoke::Invoker;
use project::adapter::Resolver;
use project::handler::{Anchor, Render};
use project::seam::{Source, Target};
use serde::Serialize;
use tracing::Instrument as _;

use self::output::{ErrorBody, Exit, emit, write_error_text};
pub use self::output::{Format, render_failure, render_success};

mod adapter;
mod archive;
mod journal;
mod output;
mod plan;
mod registry;
pub mod selectors;
mod slice;
mod source;
mod target;

/// One-line application description.
const ABOUT: &str = "Deterministic primitives for spec-driven development";

/// Arguments shared by every command route.
#[derive(Clone, Copy, Debug, Args)]
pub struct Globals {
    /// Output format.
    #[arg(long, env = "EMERY_FORMAT", default_value = "text")]
    pub format: Format,
}

/// Flags for `emery init`.
#[derive(Debug, Args)]
struct InitArgs {
    /// Adapter identifier or local component path.
    #[arg(conflicts_with = "workspace")]
    adapter: Option<String>,
    /// Project name.
    #[arg(long)]
    name: Option<String>,
    /// Project description.
    #[arg(long)]
    description: Option<String>,
    /// Scaffold a registry-only workspace.
    #[arg(long)]
    workspace: bool,
    /// Comma-separated target platforms.
    #[arg(long, conflicts_with = "workspace")]
    platforms: Option<String>,
    /// Re-enter initialization to update the Emery version pin.
    #[arg(long, conflicts_with_all = ["adapter", "workspace", "name", "description"])]
    upgrade: bool,
}

#[derive(Clone, Copy)]
struct NamespaceHelp {
    path: &'static [&'static str],
    metadata: Namespace,
}

impl NamespaceHelp {
    const fn new(path: &'static [&'static str], about: &'static str) -> Self {
        Self {
            path,
            metadata: Namespace::new().about(about),
        }
    }
}

const NAMESPACE_HELP: &[NamespaceHelp] = &[
    NamespaceHelp::new(
        &["adapter"],
        "Adapter component cache operations. `add` seeds a local `.wasm` component into the project component cache — pre-init, axis-neutral — so bare bindings (project target, plan sources) resolve it",
    ),
    NamespaceHelp::new(
        &["source"],
        "Source adapter operations (workflow contract) — debug/breakout surface; `plan author` and `slice refine` run these steps themselves. Source adapters provide `extract` + `survey` capabilities and resolve to a single `.wasm` component: the global store entry for pinned identities, the seeded project component cache for bare names",
    ),
    NamespaceHelp::new(
        &["target"],
        "Target adapter operations (workflow contract) — debug/breakout surface; the build and merge orchestrations resolve targets themselves. Target adapters provide `guidance` + `build` + `merge` capabilities and resolve to a single `.wasm` component: the global store entry for pinned identities, the seeded project component cache for bare names",
    ),
    NamespaceHelp::new(
        &["slice"],
        "Slice lifecycle operations — one `refine → build → merge` loop",
    ),
    NamespaceHelp::new(&["slice", "model"], "Read-only viewer over a slice's `model.yaml`"),
    NamespaceHelp::new(&["slice", "merge"], "Spec-merge operations for a slice"),
    NamespaceHelp::new(
        &["archive"],
        "Slice-archive cache maintenance. The archived slice folders under `.emery/archive/` are a prunable convenience cache; `prune` reclaims disk by retention bound",
    ),
    NamespaceHelp::new(&["plan"], "Executable plan operations — `plan.yaml` lifecycle"),
    NamespaceHelp::new(
        &["journal"],
        "Workflow journal at `.emery/journal.jsonl`. `emit` is a guarded front door onto the closed §Observability event taxonomy — it appends one well-formed line, minting no event kinds of its own",
    ),
    NamespaceHelp::new(&["registry"], "Platform registry at `registry.yaml` (repo root)"),
];

/// Emery's command output and error projection.
#[derive(Clone, Copy, Debug, Default)]
pub struct EmeryProjector;

impl<T> Projector<T, project::handler::Error, error::Error, Globals> for EmeryProjector
where
    T: Render + Serialize + Send + 'static,
{
    type Error = error::Error;

    fn project(
        &self, outcome: Outcome<T, project::handler::Error, error::Error>, globals: &Globals,
    ) -> Result<CommandResponse, Self::Error> {
        match outcome {
            Outcome::Output(output) => {
                Ok(CommandResponse::success(encode(globals.format, &output, |w, v| v.render(w))?))
            }
            Outcome::Operation(operation) => operation_response(globals.format, operation),
            Outcome::Decode(error) => Ok(error_response(globals.format, &error)?),
        }
    }

    fn project_failure(&self, error: Self::Error, globals: &Globals) -> CommandResponse {
        failure_response(globals.format, &error)
    }
}

/// Buffer one [`emit`] rendering of `value` for a `CommandResponse`
/// channel.
fn encode<T: Serialize>(
    format: Format, value: &T,
    text: impl FnOnce(&mut dyn std::io::Write, &T) -> std::io::Result<()>,
) -> Result<Vec<u8>, error::Error> {
    let mut bytes = Vec::new();
    emit(&mut bytes, format, value, text)?;
    Ok(bytes)
}

fn error_response(format: Format, error: &error::Error) -> Result<CommandResponse, error::Error> {
    let body = ErrorBody::from(error);
    let stderr = encode(format, &body, write_error_text)?;
    Ok(CommandResponse::failure(stderr, Exit::from(error).code()))
}

/// [`render_failure`] mapped onto a `CommandResponse` — the terminal
/// fallback (a plain exit-1 line) lives in one place.
fn failure_response(format: Format, error: &error::Error) -> CommandResponse {
    let (stderr, code) = render_failure(format, error);
    CommandResponse::failure(stderr, code)
}

fn operation_response(
    format: Format, error: project::handler::Error,
) -> Result<CommandResponse, error::Error> {
    match error {
        project::handler::Error::Core(source) => error_response(format, &source),
        project::handler::Error::Report { body, source } => {
            let stdout = encode(format, &body, |w, v| v.render(w))?;
            let mut response = error_response(format, &source)?;
            response.stdout = stdout;
            Ok(response)
        }
    }
}

/// Assemble the complete Emery command router.
///
/// # Errors
///
/// Returns a deterministic route or argument conflict.
#[expect(
    clippy::too_many_lines,
    reason = "the exhaustive typed route inventory is one auditable assembly"
)]
pub fn router<P>(invoker: Invoker<P>) -> Result<Router<P, Globals>, BuildError>
where
    P: Provider + Anchor + Model + Resolver + Source + Target,
{
    // The version display carries the embedded first-party adapter
    // train alongside the host SemVer — the two axes version
    // independently (RFC-77 D1), so operators can read the train a
    // bare name auto-pins to without opening source.
    static VERSION: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
        format!(
            "{} (adapters {})",
            env!("CARGO_PKG_VERSION"),
            project::adapter::FIRST_PARTY_ADAPTER_TRAIN
        )
    });
    let command = clap::Command::new("emery").version(VERSION.as_str()).about(ABOUT);
    let mut router = RouterBuilder::new(command, invoker)
        .completions(
            Completions::new()
                .about("Print a shell-completion script for `<shell>` to stdout")
                .long_about("Print a shell-completion script for `<shell>` to stdout.\n\nPipe into your shell's completion directory (e.g. `emery completions zsh > ~/.zsh/_emery`). Generated via `clap_complete`; the output tracks the live clap surface so every new verb is auto-discovered."),
        );

    macro_rules! route {
        ($path:expr, $args:ty, $operation:ty, $about:literal) => {
            router = router.route(
                $path,
                run::<$args, $operation>().about($about).project_with(EmeryProjector),
            );
        };
        ($path:expr, $args:ty, $operation:ty, $about:literal, $long_about:literal) => {
            router = router.route(
                $path,
                run::<$args, $operation>()
                    .about($about)
                    .long_about($long_about)
                    .project_with(EmeryProjector),
            );
        };
    }

    route!(
        ["init"],
        InitArgs,
        project::init::handlers::Init,
        "Initialize .emery/ in a project",
        "Initialize .emery/ in a project.\n\nPass `<adapter>` (first-party shorthand, package reference, or local component path) for a regular project, or `--workspace` for a registry-only workspace. The two are mutually exclusive — clap enforces the conflict and exits `2` with its standard parse-error diagnostic. A missing `<adapter>` fails typed with `init-adapter-required` (exit 2). Re-running `init` in an already-initialized project changes nothing and exits 0 routing to `emery init --upgrade`."
    );
    route!(
        ["adapter", "add"],
        adapter::AddArgs,
        project::adapter::handlers::AdapterAdd,
        "Seed a local `.wasm` component into the project component cache",
        "Seed a local `.wasm` component into the project component cache.\n\nMirrors the component to `<project-cache>/components/<name>.wasm` (the kebab name derives from the filename) and stamps a per-component provenance sidecar. Pre-init and axis-neutral: `.emery/` need not exist and the component's exports are not inspected — the bare binding that later resolves the name (project target or plan source) supplies the expected axis. Re-seeding the same name replaces the entry; the explicit command is the approval act."
    );
    route!(
        ["source", "resolve"],
        source::ResolveArgs,
        project::adapter::handlers::SourceResolve,
        "Resolve a source adapter by kebab name",
        "Resolve a source adapter by kebab name.\n\nResolves the single `.wasm` component: the global store entry for a pinned identity, else the seeded project component cache for a bare name. Emits the resolved component path plus the axis's closed operation set."
    );
    route!(
        ["source", "survey"],
        source::SurveyArgs,
        ::change::source::Survey,
        "Run a source adapter's `survey` against a plan-bound source and merge the resulting lead set into `discovery.md`",
        "Run a source adapter's `survey` against a plan-bound source and merge the resulting lead set into `discovery.md`.\n\nResolves `<source>` against `plan.yaml.sources.<key>` (not the adapter name) and drives the bound source adapter's collapsed survey orchestration in the engine guest — one call covering the source dispatch, `leads.md` validation, and the `discovery.md` merge."
    );
    route!(
        ["source", "extract"],
        source::ExtractArgs,
        ::slice::source::Extract,
        "Run a source adapter's `extract` for one `(source, lead)` pair and persist the resulting Evidence to `.emery/slices/<slice>/evidence/<source>.yaml`",
        "Run a source adapter's `extract` for one `(source, lead)` pair and persist the resulting Evidence to `.emery/slices/<slice>/evidence/<source>.yaml`.\n\nResolves `<source>` against `plan.yaml.sources.<key>` (not the adapter name) and drives the bound source adapter's collapsed extract orchestration in the engine guest — one call covering the source dispatch, the typed Evidence validation, and the persist."
    );
    route!(
        ["target", "resolve"],
        target::ResolveArgs,
        project::adapter::handlers::TargetResolve,
        "Resolve a target adapter"
    );
    route!(
        ["slice", "list"],
        slice::ListArgs,
        ::slice::handlers::List,
        "List every slice under `.emery/slices/` with its lifecycle status and target"
    );
    route!(
        ["slice", "validate"],
        slice::ValidateArgs,
        ::slice::handlers::Validate,
        "Validate a slice's artifacts against adapter validation rules"
    );
    route!(
        ["slice", "provenance"],
        slice::ProvenanceArgs,
        ::slice::handlers::Provenance,
        "Project the audit-only provenance view from the slice's `model.yaml`. Provenance is carried inline in `model.yaml`; this reshapes it on demand and never reads or writes a `provenance.yaml` file"
    );
    route!(
        ["slice", "model", "show"],
        slice::ModelShowArgs,
        ::slice::handlers::ModelShow,
        "Render the persisted `model.yaml` — concise text view, or the model serialised verbatim under `--format json`"
    );
    route!(
        ["slice", "refine"],
        slice::RefineArgs,
        ::slice::handlers::Refine,
        "Refine one named plan entry's slice to `refined` in the engine guest: slice create (re-entry safe), per-binding extract fan-out, the synthesis judgment leg, the persist tail, validate, and the `refined` transition — the `/emery:refine` breakout outside the execute loop",
        "Refine one named plan entry's slice to `refined` in the engine guest: slice create (re-entry safe), per-binding extract fan-out, the synthesis judgment leg, the persist tail, validate, and the `refined` transition — the `/emery:refine` breakout outside the execute loop.\n\nActs on the named slice directly against a `pending` or `in-progress` plan entry (the standalone `slice build <name>` posture); never advances per-entry status, and refuses a `done` entry.\n\nGuest-only. The native binary refuses this verb — natively the phase is driven by the `/emery:refine` skill."
    );
    route!(
        ["slice", "build"],
        slice::BuildArgs,
        ::slice::handlers::Build,
        "Build a slice through its bound target adapter's `build` operation and gate the `built` transition",
        "Build a slice through its bound target adapter's `build` operation and gate the `built` transition.\n\nResolves the target from the slice's `metadata.yaml`, then drives the collapsed build orchestration in the engine guest: request assembly and schema gate, the target-seam dispatch, the report gates (`target-build-*` aborts), the `slice.build.*` events, and the `Refined → Built` transition. The target guest owns only code generation."
    );
    route!(
        ["slice", "merge", "run"],
        slice::MergeRunArgs,
        ::slice::handlers::MergeRun,
        "Merge all delta specs for the slice into baseline and archive the slice"
    );
    route!(
        ["slice", "merge", "preview"],
        slice::MergePreviewArgs,
        ::slice::handlers::Preview,
        "Show the merge operations that would be applied, without writing"
    );
    route!(
        ["slice", "merge", "conflict-check"],
        slice::ConflictCheckArgs,
        ::slice::handlers::ConflictCheck,
        "Report `type: modified` baselines modified after this slice's `defined_at`"
    );
    route!(
        ["slice", "drop"],
        slice::DropArgs,
        ::slice::handlers::Drop,
        "Transition a slice to `dropped` and archive it"
    );
    route!(
        ["archive", "prune"],
        archive::PruneArgs,
        ::slice::handlers::Prune,
        "Prune archived slice folders under `.emery/archive/` that fall outside the supplied retention bounds",
        "Prune archived slice folders under `.emery/archive/` that fall outside the supplied retention bounds.\n\nThe archive is a prunable convenience cache, not the system of record — git history of `.emery/specs/` plus the `slice.archive.created` journal entries are. At least one of `--keep` / `--older-than` is required; a folder is pruned when it falls outside the newest-`--keep` window or is older than `--older-than` days."
    );
    route!(
        ["plan", "validate"],
        plan::ValidateArgs,
        ::change::plan::handlers::Validate,
        "Validate plan.yaml (structure + plan/change consistency)",
        "Validate plan.yaml (structure + plan/change consistency).\n\nIncludes the three health diagnostics — `cycle-in-depends-on`, `orphan-source`, and `stale-workspace-clone` — alongside the base shape rules."
    );
    route!(
        ["plan", "next"],
        plan::NextArgs,
        ::change::plan::handlers::Next,
        "Return the active in-progress entry, or transition the next eligible `Pending` entry to `InProgress` and return it. `plan next` is the only writer of per-entry `in-progress` (workflow §CLI surface)"
    );
    route!(
        ["plan", "status"],
        plan::StatusArgs,
        ::change::plan::handlers::Status,
        "Read-only projection of the plan's execution state into a deterministic `next-action` — `refine|build|merge <slice>`, `stop <reason>`, or `drained`",
        "Read-only projection of the plan's execution state into a deterministic `next-action` — `refine|build|merge <slice>`, `stop <reason>`, or `drained`.\n\nProjects `plan.yaml` entries, the candidate slice's `metadata.yaml` lifecycle (slot-aware in workspace mode), and the journal tail. Stop reasons (`plan-not-approved`, `refine-failed`, `build-failed`, `merge-conflict`, `slice-dropped`, `merge-incomplete`, `stuck`) are classified from `slice.synthesize.failed` / `slice.build.failed` / `slice.merge.failed` journal events scoped to the active entry's claim window. Writes nothing — `plan next` stays the only writer of per-entry `in-progress`."
    );
    route!(
        ["plan", "add"],
        plan::AddArgs,
        ::change::plan::handlers::Add,
        "Add a new plan entry (status: pending)"
    );
    route!(
        ["plan", "amend"],
        plan::AmendArgs,
        ::change::plan::handlers::Amend,
        "Edit non-status fields on an existing plan entry",
        "Edit non-status fields on an existing plan entry.\n\nThree orthogonal flag families operate on `sources`:\n\n- `--sources <binding>` (with `num_args = 0..`) replaces the slice's `sources` array wholesale.\n- `--add-source <binding>` (repeatable) adds a single binding.\n- `--remove-source <key>` (repeatable) removes a binding by key; fails with `plan-binding-not-found` when no binding matches.\n\n`--add-source` and `--remove-source` apply after `--sources`, so wholesale replacement plus targeted edits can be combined in a single invocation when needed."
    );
    route!(
        ["plan", "remove"],
        plan::RemoveArgs,
        ::change::plan::handlers::Remove,
        "Remove a pending plan entry while the plan is still replaceable (`lifecycle: pending` and every entry `pending`). Gate 1 curation only — defers a lead without re-surveying `discovery.md`"
    );
    route!(
        ["plan", "approve"],
        plan::ApproveArgs,
        ::change::plan::handlers::Approve,
        "Stamp Gate 1 — transition the active plan's lifecycle `pending → approved`",
        "Stamp Gate 1 — transition the active plan's lifecycle `pending → approved`.\n\nNameless: there is exactly one active `plan.yaml`, so no selector is needed. Operator-only — `/emery:plan` MUST NOT call this verb; skill bodies stop at `pending` and print the literal `emery plan approve` command in their closing hint for the operator to run. Approving an already-approved plan is an idempotent no-op (no disk write, no journal event)."
    );
    route!(
        ["plan", "transition"],
        plan::TransitionArgs,
        ::change::plan::handlers::Transition,
        "Apply a validated per-entry status transition",
        "Apply a validated per-entry status transition.\n\n`<name>` is a plan-entry name and `<target>` is `done` — the per-entry close; the `/emery:merge` skill is the canonical caller. `--undo` walks one rung backwards instead. Plan-level Gate 1 is `emery plan approve`.\n\nPer-entry `pending` is written only by `plan add` / `plan amend`; per-entry `in-progress` is written only by `plan next`. v1 has no per-entry `blocked`, `failed`, or `skipped` state — build failures and merge conflicts leave the active entry `in-progress`."
    );
    route!(
        ["plan", "author"],
        plan::AuthorArgs,
        ::change::plan::handlers::Author,
        "Author a plan end-to-end in the engine guest: scaffold `plan.yaml`, survey every bound source into `discovery.md`, reconcile the leads into `plan.yaml.slices[]` through the judgment leg, persist the Gate 1 prose (`change.md`, `discovery.md`'s `## Summary` and `## Source inventory`), validate, and exit at `pending` with the literal Gate 1 transition hint",
        "Author a plan end-to-end in the engine guest: scaffold `plan.yaml`, survey every bound source into `discovery.md`, reconcile the leads into `plan.yaml.slices[]` through the judgment leg, persist the Gate 1 prose (`change.md`, `discovery.md`'s `## Summary` and `## Source inventory`), validate, and exit at `pending` with the literal Gate 1 transition hint.\n\nGuest-only through the composed-deployment leg: the `/emery:plan` skill invokes this single verb and relays its output."
    );
    route!(
        ["plan", "execute"],
        plan::ExecuteArgs,
        ::change::plan::handlers::Execute,
        "Run the drained execute loop in the engine guest: claim → refine → build → merge per entry until the plan projects `drained` or a stop condition halts it (exit 2, `plan-execute-stopped`)",
        "Run the drained execute loop in the engine guest: claim → refine → build → merge per entry until the plan projects `drained` or a stop condition halts it (exit 2, `plan-execute-stopped`).\n\nGuest-only through the composed-deployment leg: the loop holds the create-exclusive `.emery/guest.lock` marker (guest-vs-guest refusal only) while it drives the phases."
    );
    route!(
        ["plan", "archive"],
        plan::ArchiveArgs,
        ::change::plan::handlers::Archive,
        "Archive the current plan to `.emery/archive/plans/<name>-<YYYYMMDD>.yaml`"
    );
    route!(
        ["journal", "emit"],
        journal::EmitArgs,
        project::journal::handlers::Emit,
        "Append one event to `.emery/journal.jsonl`",
        "Append one event to `.emery/journal.jsonl`.\n\n`<event-id>` names a variant in the closed workflow §Observability event taxonomy (e.g. `source.execution.agent`); `--payload` carries that variant's fields as a JSON object. The taxonomy is the payload schema — a single serde round-trip validates both the id and the fields. An unknown id exits `2` with `journal-emit-unknown-event`; a payload that fails the variant's field schema exits `2` with `journal-emit-payload-schema`. On success the CLI stamps a second-precision UTC timestamp and appends exactly one line."
    );
    route!(
        ["journal", "show"],
        journal::ShowArgs,
        project::journal::handlers::Show,
        "Read events from `.emery/journal.jsonl` in append order",
        "Read events from `.emery/journal.jsonl` in append order.\n\nRead-only: emits no journal event and writes nothing. Text mode prints the canonical JSONL lines — one `{ timestamp, event, payload }` object per event, pipeable — while `--format json` wraps the same events in the standard envelope. Blank and unparseable lines are skipped, matching every other journal reader; a missing journal yields no events."
    );
    route!(
        ["registry", "validate"],
        registry::ValidateArgs,
        project::registry::handlers::Validate,
        "Validate `registry.yaml` shape. Absent file exits 0"
    );
    route!(
        ["registry", "add"],
        registry::AddArgs,
        project::registry::handlers::Add,
        "Append a new project entry to `registry.yaml`. Creates the file when absent"
    );
    route!(
        ["registry", "remove"],
        registry::RemoveArgs,
        project::registry::handlers::Remove,
        "Remove an existing project entry. Warns when `plan.yaml` references it"
    );

    for help in NAMESPACE_HELP {
        router = router.namespace(help.path.iter().copied(), help.metadata);
    }
    router.build()
}

/// Run one routed invocation (`argv[0]` is the binary name) under the
/// `emery.command` span.
///
/// The span carries only the bounded verb label and the response exit
/// code — never the full argv, which may embed operator prose (e.g.
/// `plan author --intent …`). Both deployments route through here: the
/// native host's command entry and the engine guest's `wasi:cli/run`
/// exporter.
pub async fn execute<P>(router: &Router<P, Globals>, argv: Vec<String>) -> CommandResponse
where
    P: Provider + Anchor + Model + Resolver + Source + Target,
{
    let span = tracing::info_span!(
        "emery.command",
        command = %label(&argv),
        exit = tracing::field::Empty,
    );
    async {
        let response = router.execute(argv).await;
        tracing::Span::current().record("exit", response.exit);
        response
    }
    .instrument(span)
    .await
}

/// The bounded span label: the first two non-flag tokens after the
/// binary name (`plan author`, `slice build`).
fn label(argv: &[String]) -> String {
    let words: Vec<&str> = argv
        .iter()
        .skip(1)
        .filter(|arg| !arg.starts_with('-'))
        .take(2)
        .map(String::as_str)
        .collect();
    words.join(" ")
}

macro_rules! convert {
    // The destructuring pattern is exhaustive on purpose: a new clap
    // flag missing from the field list is a compile error, not a
    // silently dropped argument.
    ($args:path => $input:path {}) => {
        impl TryFrom<$args> for $input {
            type Error = error::Error;

            fn try_from(args: $args) -> Result<Self, Self::Error> {
                let $args {} = args;
                Ok(Self {})
            }
        }
    };
    ($args:path => $input:path { $($field:ident),* $(,)? }) => {
        impl TryFrom<$args> for $input {
            type Error = error::Error;

            fn try_from(args: $args) -> Result<Self, Self::Error> {
                let $args { $($field),* } = args;
                Ok(Self { $($field),* })
            }
        }
    };
}

convert!(adapter::AddArgs => project::adapter::handlers::AddInput { component, project_dir });
convert!(source::ResolveArgs => project::adapter::handlers::ResolveInput { value, project_dir });
convert!(target::ResolveArgs => project::adapter::handlers::ResolveInput { value, project_dir });
convert!(source::SurveyArgs => ::change::source::SurveyInput { source, plan });
convert!(source::ExtractArgs => ::slice::source::ExtractInput { source, lead, slice });
convert!(slice::ListArgs => ::slice::handlers::ListInput {});
convert!(slice::ValidateArgs => ::slice::handlers::ValidateInput { name });
convert!(slice::ProvenanceArgs => ::slice::handlers::ProvenanceInput { name });
convert!(slice::ModelShowArgs => ::slice::handlers::ModelShowInput { name });
convert!(slice::RefineArgs => ::slice::handlers::RefineInput { name });
convert!(slice::BuildArgs => ::slice::handlers::BuildInput { name });
convert!(slice::MergeRunArgs => ::slice::handlers::MergeRunInput { name, allow_composition_replace });
convert!(slice::MergePreviewArgs => ::slice::handlers::PreviewInput { name });
convert!(slice::ConflictCheckArgs => ::slice::handlers::ConflictCheckInput { name });
convert!(slice::DropArgs => ::slice::handlers::DropInput { name, reason });
convert!(archive::PruneArgs => ::slice::handlers::PruneInput { keep, older_than, dry_run });
convert!(plan::ValidateArgs => ::change::plan::handlers::ValidateInput {});
convert!(plan::NextArgs => ::change::plan::handlers::NextInput {});
convert!(plan::StatusArgs => ::change::plan::handlers::StatusInput {});
convert!(plan::ExecuteArgs => ::change::plan::handlers::ExecuteInput {});
convert!(plan::AddArgs => ::change::plan::handlers::AddInput { name, depends_on, sources, description, project, context, authority_override });
convert!(plan::AmendArgs => ::change::plan::handlers::AmendInput { name, depends_on, sources, add_source, remove_source, divergence, description, project, context, authority_override, clear_authority_override, clear_authority_overrides });
convert!(plan::RemoveArgs => ::change::plan::handlers::RemoveInput { name });
convert!(plan::ApproveArgs => ::change::plan::handlers::ApproveInput { actor });
convert!(plan::TransitionArgs => ::change::plan::handlers::TransitionInput { name, target, undo });
convert!(plan::AuthorArgs => ::change::plan::handlers::AuthorInput { name, sources, intent });
convert!(plan::ArchiveArgs => ::change::plan::handlers::ArchiveInput { force });
convert!(journal::EmitArgs => project::journal::handlers::EmitInput { event, payload });
convert!(journal::ShowArgs => project::journal::handlers::ShowInput { filter, limit });
convert!(registry::ValidateArgs => project::registry::handlers::ValidateInput {});
convert!(registry::AddArgs => project::registry::handlers::AddInput { name, url, adapter, description });
convert!(registry::RemoveArgs => project::registry::handlers::RemoveInput { name });

convert!(InitArgs => project::init::handlers::InitInput { adapter, name, description, workspace, platforms, upgrade });

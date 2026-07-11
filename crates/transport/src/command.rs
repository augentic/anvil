//! Typed command grammar, conversions, and Specify projection policy.

use std::fs;
use std::path::Path;

use omnia_guest::Model;
use omnia_guest::api::Provider;
use omnia_guest::api::command::{
    BuildError, CommandResponse, Completions, Namespace, Outcome, Projector, Router, RouterBuilder,
    run,
};
use omnia_guest::api::invoke::{CallContext, Invoker};
use omnia_guest::api::operation::Operation;
use serde::Serialize;
use workflow::adapter::Resolver;
use workflow::handler::{Anchor, Render};
use workflow::seam::{SourceSeam, TargetSeam};

use crate::args::{
    ABOUT, Globals, InitArgs, UpgradeArgs, adapters, archive, journal, plan, plugins, registry,
    slice, source, target, workspace,
};
use crate::output::{ErrorBody, Exit, Format, emit, write_error_text};

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

    const fn detailed(
        path: &'static [&'static str], about: &'static str, long_about: &'static str,
    ) -> Self {
        Self {
            path,
            metadata: Namespace::new().about(about).long_about(long_about),
        }
    }
}

const NAMESPACE_HELP: &[NamespaceHelp] = &[
    NamespaceHelp::new(
        &["adapters"],
        "Global adapter-store provisioning. `sync` is the explicit hydration trigger: it hydrates every pinned identity the project declares (`project.yaml` plus `plan.yaml` source pins) into the global store and prints the resolved set. Native provisioning verb — never runs in the workflow guest",
    ),
    NamespaceHelp::new(
        &["source"],
        "Source adapter operations (workflow contract). Source adapters provide `extract` + `survey` capabilities and resolve to a single `.wasm` component: the global store entry for pinned identities, the development release build for bare names",
    ),
    NamespaceHelp::new(
        &["target"],
        "Target adapter operations (workflow contract). Target adapters provide `guidance` + `build` + `merge` capabilities and resolve to a single `.wasm` component: the global store entry for pinned identities, the development release build for bare names",
    ),
    NamespaceHelp::new(
        &["slice"],
        "Slice lifecycle operations — one `refine → build → merge` loop",
    ),
    NamespaceHelp::new(&["slice", "model"], "Read-only viewer over a slice's `model.yaml`"),
    NamespaceHelp::new(&["slice", "merge"], "Spec-merge operations for a slice"),
    NamespaceHelp::new(&["slice", "task"], "Tasks-list operations for a slice"),
    NamespaceHelp::new(
        &["archive"],
        "Slice-archive cache maintenance. The archived slice folders under `.specify/archive/` are a prunable convenience cache; `prune` reclaims disk by retention bound",
    ),
    NamespaceHelp::new(&["plan"], "Executable plan operations — `plan.yaml` lifecycle"),
    NamespaceHelp::new(
        &["journal"],
        "Workflow journal at `.specify/journal.jsonl`. `emit` is a guarded front door onto the closed §Observability event taxonomy — it appends one well-formed line, minting no event kinds of its own",
    ),
    NamespaceHelp::new(&["registry"], "Platform registry at `registry.yaml` (repo root)"),
    NamespaceHelp::new(&["workspace"], "Materialise and manage registry peers under `workspace/`"),
    NamespaceHelp::detailed(
        &["plugins"],
        "Inspect and invalidate the Cursor plugin cache",
        "Inspect and invalidate the Cursor plugin cache.\n\nBootstrap verb: operates on `$CURSOR_HOME/plugins/cache/<name>/` and the marketplace manifest, not a project, so it never loads project config. `doctor` reports per-plugin drift (read-only); `refresh` clears the marketplace-scoped cache after `--yes` and prints a restart instruction. The CLI never restarts Cursor.",
    ),
];

/// Specify's command output and error projection.
#[derive(Clone, Copy, Debug, Default)]
pub struct SpecifyProjector;

impl<T> Projector<T, workflow::handler::Error, error::Error, Globals> for SpecifyProjector
where
    T: Render + Serialize + Send + 'static,
{
    type Error = error::Error;

    fn project(
        &self, outcome: Outcome<T, workflow::handler::Error, error::Error>, globals: &Globals,
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

/// [`error_response`] with rendering failures collapsed onto a plain
/// exit-1 line — the terminal fallback when the envelope itself cannot
/// be produced.
fn failure_response(format: Format, error: &error::Error) -> CommandResponse {
    error_response(format, error)
        .unwrap_or_else(|fallback| CommandResponse::failure(format!("error: {fallback}\n"), 1))
}

fn operation_response(
    format: Format, error: workflow::handler::Error,
) -> Result<CommandResponse, error::Error> {
    match error {
        workflow::handler::Error::Core(source) => error_response(format, &source),
        workflow::handler::Error::Report { body, source } => {
            let stdout = encode(format, &body, |w, v| v.render(w))?;
            let mut response = error_response(format, &source)?;
            response.stdout = stdout;
            Ok(response)
        }
    }
}

/// Assemble the complete Specify command router.
///
/// # Errors
///
/// Returns a deterministic route or argument conflict.
#[expect(
    clippy::too_many_lines,
    reason = "the exhaustive typed route inventory is one auditable assembly"
)]
pub fn router<P>(
    invoker: Invoker<P>,
    preflight: impl Fn(&Globals) -> Result<(), error::Error> + Send + Sync + 'static,
) -> Result<Router<P, Globals>, BuildError>
where
    P: Provider + Anchor + Model + Resolver + SourceSeam + TargetSeam,
{
    let command = clap::Command::new("specify").version(env!("CARGO_PKG_VERSION")).about(ABOUT);
    let mut router = RouterBuilder::new(command, invoker)
        .completions(
            Completions::new()
                .about("Print a shell-completion script for `<shell>` to stdout")
                .long_about("Print a shell-completion script for `<shell>` to stdout.\n\nPipe into your shell's completion directory (e.g. `specify completions zsh > ~/.zsh/_specify`). Generated via `clap_complete`; the output tracks the live clap surface so every new verb is auto-discovered."),
        )
        .before_dispatch(move |globals: &Globals| {
            check_plan_dir(globals.plan_dir.as_deref())
                .and_then(|()| preflight(globals))
                .err()
                .map(|error| failure_response(globals.format, &error))
        });

    macro_rules! route {
        ($path:expr, $args:ty, $operation:ty, $about:literal) => {
            router = router.route(
                $path,
                run::<$args, $operation>().about($about).project_with(SpecifyProjector),
            );
        };
        ($path:expr, $args:ty, $operation:ty, $about:literal, $long_about:literal) => {
            router = router.route(
                $path,
                run::<$args, $operation>()
                    .about($about)
                    .long_about($long_about)
                    .project_with(SpecifyProjector),
            );
        };
    }

    route!(
        ["init"],
        InitArgs,
        workflow::init::handlers::Scaffold,
        "Initialize .specify/ in a project",
        "Initialize .specify/ in a project.\n\nPass `<adapter>` (first-party shorthand, local path, or URL) for a regular project, or `--workspace` for a registry-only workspace. The two are mutually exclusive — clap enforces the conflict and exits `2` with its standard parse-error diagnostic. A missing `<adapter>` reaches the native elicitation layer: prompted on a TTY, the typed `init-adapter-required` (exit 2) everywhere else."
    );
    route!(
        ["source", "resolve"],
        source::ResolveArgs,
        workflow::adapter::handlers::SourceResolve,
        "Resolve a source adapter by kebab name",
        "Resolve a source adapter by kebab name.\n\nResolves the single `.wasm` component: the global store entry for a pinned identity, else the project component cache / development release build for a bare name. Emits the resolved component path plus the axis's closed operation set."
    );
    route!(
        ["source", "survey"],
        source::SurveyArgs,
        workflow::orchestrate::handlers::Survey,
        "Run a source adapter's `survey` against a plan-bound source and merge the resulting lead set into `discovery.md`",
        "Run a source adapter's `survey` against a plan-bound source and merge the resulting lead set into `discovery.md`.\n\nResolves `<source>` against `plan.yaml.sources.<key>` (not the adapter name) and drives the bound source adapter's collapsed survey orchestration in the workflow guest — one call covering the source dispatch, `leads.md` validation, and the `discovery.md` merge."
    );
    route!(
        ["source", "extract"],
        source::ExtractArgs,
        workflow::orchestrate::handlers::Extract,
        "Run a source adapter's `extract` for one `(source, lead)` pair and persist the resulting Evidence to `.specify/slices/<slice>/evidence/<source>.yaml`",
        "Run a source adapter's `extract` for one `(source, lead)` pair and persist the resulting Evidence to `.specify/slices/<slice>/evidence/<source>.yaml`.\n\nResolves `<source>` against `plan.yaml.sources.<key>` (not the adapter name) and drives the bound source adapter's collapsed extract orchestration in the workflow guest — one call covering the source dispatch, the Evidence schema gate (`schemas/evidence.schema.json`), and the persist."
    );
    route!(
        ["target", "resolve"],
        target::ResolveArgs,
        workflow::adapter::handlers::TargetResolve,
        "Resolve a target adapter"
    );
    route!(
        ["slice", "create"],
        slice::CreateArgs,
        workflow::slice::handlers::Create,
        "Create a new slice directory with an initial `metadata.yaml`"
    );
    route!(
        ["slice", "validate"],
        slice::ValidateArgs,
        workflow::slice::handlers::Validate,
        "Validate a slice's artifacts against adapter validation rules"
    );
    route!(
        ["slice", "provenance"],
        slice::ProvenanceArgs,
        workflow::slice::handlers::Provenance,
        "Project the audit-only provenance view from the slice's `model.yaml`. Provenance is carried inline in `model.yaml`; this reshapes it on demand and never reads or writes a `provenance.yaml` file"
    );
    route!(
        ["slice", "model", "show"],
        slice::ModelShowArgs,
        workflow::slice::handlers::ModelShow,
        "Render the persisted `model.yaml` — concise text view, or the model serialised verbatim under `--format json`"
    );
    route!(
        ["slice", "refine"],
        slice::RefineArgs,
        workflow::orchestrate::handlers::Refine,
        "Refine one named plan entry's slice to `refined` in the workflow guest: slice create (re-entry safe), per-binding extract fan-out, the synthesis judgment leg, the persist tail, validate, and the `refined` transition — the `/spec:refine` breakout outside the execute loop",
        "Refine one named plan entry's slice to `refined` in the workflow guest: slice create (re-entry safe), per-binding extract fan-out, the synthesis judgment leg, the persist tail, validate, and the `refined` transition — the `/spec:refine` breakout outside the execute loop.\n\nActs on the named slice directly against a `pending` or `in-progress` plan entry (the standalone `slice build <name>` posture); never advances per-entry status, and refuses a `done` entry.\n\nGuest-only. The native binary refuses this verb — natively the phase is driven by the `/spec:refine` skill."
    );
    route!(
        ["slice", "build"],
        slice::BuildArgs,
        workflow::orchestrate::handlers::Build,
        "Build a slice through its bound target adapter's `build` operation and gate the `built` transition",
        "Build a slice through its bound target adapter's `build` operation and gate the `built` transition.\n\nResolves the target from the slice's `metadata.yaml`, then drives the collapsed build orchestration in the workflow guest: request assembly and schema gate, the target-seam dispatch, the report gates (`target-build-*` aborts), the `slice.build.*` events, and the `Refined → Built` transition. The target guest owns only code generation."
    );
    route!(
        ["slice", "merge", "run"],
        slice::MergeRunArgs,
        workflow::orchestrate::handlers::MergeRun,
        "Merge all delta specs for the slice into baseline and archive the slice"
    );
    route!(
        ["slice", "merge", "preview"],
        slice::MergePreviewArgs,
        workflow::slice::handlers::Preview,
        "Show the merge operations that would be applied, without writing"
    );
    route!(
        ["slice", "merge", "conflict-check"],
        slice::ConflictCheckArgs,
        workflow::slice::handlers::ConflictCheck,
        "Report `type: modified` baselines modified after this slice's `defined_at`"
    );
    route!(
        ["slice", "task", "progress"],
        slice::TaskProgressArgs,
        workflow::slice::handlers::TaskProgress,
        "Report task completion counts (total, complete, pending)"
    );
    route!(
        ["slice", "task", "mark"],
        slice::TaskMarkArgs,
        workflow::slice::handlers::TaskMark,
        "Mark a task complete (idempotent — no-op if already complete)"
    );
    route!(
        ["slice", "transition"],
        slice::TransitionArgs,
        workflow::slice::handlers::Transition,
        "Transition a slice to a new lifecycle status. Note: `merged` is not a valid target — the only legal writer of `Merged` is `specify slice merge run`, which performs the spec merge, status transition, and archive move atomically"
    );
    route!(
        ["slice", "touched-specs"],
        slice::TouchedSpecsArgs,
        workflow::slice::handlers::TouchedSpecs,
        "Scan or overwrite `touched_specs` on `metadata.yaml`"
    );
    route!(
        ["slice", "overlap"],
        slice::OverlapArgs,
        workflow::slice::handlers::Overlap,
        "Report overlapping `touched_specs` with other active slices"
    );
    route!(
        ["slice", "drop"],
        slice::DropArgs,
        workflow::slice::handlers::Drop,
        "Transition a slice to `dropped` and archive it"
    );
    route!(
        ["archive", "prune"],
        archive::PruneArgs,
        workflow::slice::handlers::Prune,
        "Prune archived slice folders under `.specify/archive/` that fall outside the supplied retention bounds",
        "Prune archived slice folders under `.specify/archive/` that fall outside the supplied retention bounds.\n\nThe archive is a prunable convenience cache, not the system of record — git history of `.specify/specs/` plus the `slice.archive.created` journal entries are. At least one of `--keep` / `--older-than` is required; a folder is pruned when it falls outside the newest-`--keep` window or is older than `--older-than` days."
    );
    route!(
        ["plan", "create"],
        plan::CreateArgs,
        workflow::change::plan::handlers::Create,
        "Scaffold an empty `plan.yaml` at the repo root. Refuses to overwrite an existing plan"
    );
    route!(
        ["plan", "validate"],
        plan::ValidateArgs,
        workflow::change::plan::handlers::Validate,
        "Validate plan.yaml (structure + plan/change consistency)",
        "Validate plan.yaml (structure + plan/change consistency).\n\nIncludes the three health diagnostics — `cycle-in-depends-on`, `orphan-source`, and `stale-workspace-clone` — alongside the base shape rules."
    );
    route!(
        ["plan", "next"],
        plan::NextArgs,
        workflow::change::plan::handlers::Next,
        "Return the active in-progress entry, or transition the next eligible `Pending` entry to `InProgress` and return it. `plan next` is the only writer of per-entry `in-progress` (workflow §CLI surface)"
    );
    route!(
        ["plan", "status"],
        plan::StatusArgs,
        workflow::change::plan::handlers::Status,
        "Read-only projection of the plan's execution state into a deterministic `next-action` — `refine|build|merge <slice>`, `stop <reason>`, or `drained`",
        "Read-only projection of the plan's execution state into a deterministic `next-action` — `refine|build|merge <slice>`, `stop <reason>`, or `drained`.\n\nProjects `plan.yaml` entries, the candidate slice's `metadata.yaml` lifecycle (slot-aware in workspace mode), and the journal tail. Stop reasons (`plan-not-approved`, `refine-failed`, `build-failed`, `merge-conflict`, `slice-dropped`, `merge-incomplete`, `stuck`) are classified from `slice.synthesize.failed` / `slice.build.failed` / `slice.merge.failed` journal events scoped to the active entry's claim window. Writes nothing — `plan next` stays the only writer of per-entry `in-progress`."
    );
    route!(
        ["plan", "add"],
        plan::AddArgs,
        workflow::change::plan::handlers::Add,
        "Add a new plan entry (status: pending)"
    );
    route!(
        ["plan", "amend"],
        plan::AmendArgs,
        workflow::change::plan::handlers::Amend,
        "Edit non-status fields on an existing plan entry",
        "Edit non-status fields on an existing plan entry.\n\nThree orthogonal flag families operate on `sources`:\n\n- `--sources <binding>` (with `num_args = 0..`) replaces the slice's `sources` array wholesale.\n- `--add-source <binding>` (repeatable) adds a single binding.\n- `--remove-source <key>` (repeatable) removes a binding by key; fails with `plan-binding-not-found` when no binding matches.\n\n`--add-source` and `--remove-source` apply after `--sources`, so wholesale replacement plus targeted edits can be combined in a single invocation when needed."
    );
    route!(
        ["plan", "remove"],
        plan::RemoveArgs,
        workflow::change::plan::handlers::Remove,
        "Remove a pending plan entry while the plan is still replaceable (`lifecycle: pending` and every entry `pending`). Gate 1 curation only — defers a lead without re-surveying `discovery.md`"
    );
    route!(
        ["plan", "transition"],
        plan::TransitionArgs,
        workflow::change::plan::handlers::Transition,
        "Apply a validated status transition",
        "Apply a validated status transition.\n\nTwo transition shapes share this verb (workflow §CLI surface):\n\n- Plan-level Gate 1 stamp — `<name>` is the plan name and `<target>` is `approved`. Operator-only — `/spec:plan` MUST NOT call this verb; skill bodies stop at `pending` and print the literal `specify plan transition <name> approved` command in their closing hint for the operator to run.\n- Per-entry close — `<name>` is a plan-entry name and `<target>` is `done`. The `/spec:merge` skill is the canonical caller.\n\nPer-entry `pending` is written only by `plan add` / `plan amend`; per-entry `in-progress` is written only by `plan next`. v1 has no per-entry `blocked`, `failed`, or `skipped` state — build failures and merge conflicts leave the active entry `in-progress`."
    );
    route!(
        ["plan", "author"],
        plan::AuthorArgs,
        workflow::orchestrate::handlers::Author,
        "Author a plan end-to-end in the workflow guest: scaffold `plan.yaml` (`plan create` semantics), survey every bound source into `discovery.md`, reconcile the leads into `plan.yaml.slices[]` through the judgment leg, persist the Gate 1 prose (`change.md`, `discovery.md`'s `## Summary` and `## Source inventory`), validate, and exit at `pending` with the literal Gate 1 transition hint",
        "Author a plan end-to-end in the workflow guest: scaffold `plan.yaml` (`plan create` semantics), survey every bound source into `discovery.md`, reconcile the leads into `plan.yaml.slices[]` through the judgment leg, persist the Gate 1 prose (`change.md`, `discovery.md`'s `## Summary` and `## Source inventory`), validate, and exit at `pending` with the literal Gate 1 transition hint.\n\nGuest-only through the composed-deployment leg: the `/spec:plan` skill invokes this single verb and relays its output."
    );
    route!(
        ["plan", "execute"],
        plan::ExecuteArgs,
        workflow::orchestrate::handlers::Execute,
        "Run the drained execute loop in the workflow guest: claim → refine → build → merge per entry until the plan projects `drained` or a stop condition halts it (exit 2, `plan-execute-stopped`)",
        "Run the drained execute loop in the workflow guest: claim → refine → build → merge per entry until the plan projects `drained` or a stop condition halts it (exit 2, `plan-execute-stopped`).\n\nGuest-only through the composed-deployment leg: the loop holds the create-exclusive `.specify/guest.lock` marker (guest-vs-guest refusal only) while it drives the phases."
    );
    route!(
        ["plan", "archive"],
        plan::ArchiveArgs,
        workflow::change::plan::handlers::Archive,
        "Archive the current plan to `.specify/archive/plans/<name>-<YYYYMMDD>.yaml`"
    );
    route!(
        ["journal", "emit"],
        journal::EmitArgs,
        workflow::journal::handlers::Emit,
        "Append one event to `.specify/journal.jsonl`",
        "Append one event to `.specify/journal.jsonl`.\n\n`<event-id>` names a variant in the closed workflow §Observability event taxonomy (e.g. `source.execution.agent`); `--payload` carries that variant's fields as a JSON object. The taxonomy is the payload schema — a single serde round-trip validates both the id and the fields. An unknown id exits `2` with `journal-emit-unknown-event`; a payload that fails the variant's field schema exits `2` with `journal-emit-payload-schema`. On success the CLI stamps a second-precision UTC timestamp and appends exactly one line."
    );
    route!(
        ["journal", "show"],
        journal::ShowArgs,
        workflow::journal::handlers::Show,
        "Read events from `.specify/journal.jsonl` in append order",
        "Read events from `.specify/journal.jsonl` in append order.\n\nRead-only: emits no journal event and writes nothing. Text mode prints the canonical JSONL lines — one `{ timestamp, event, payload }` object per event, pipeable — while `--format json` wraps the same events in the standard envelope. Blank and unparseable lines are skipped, matching every other journal reader; a missing journal yields no events."
    );
    route!(
        ["registry", "validate"],
        registry::ValidateArgs,
        workflow::registry::handlers::Validate,
        "Validate `registry.yaml` shape. Absent file exits 0"
    );
    route!(
        ["registry", "add"],
        registry::AddArgs,
        workflow::registry::handlers::Add,
        "Append a new project entry to `registry.yaml`. Creates the file when absent"
    );
    route!(
        ["registry", "remove"],
        registry::RemoveArgs,
        workflow::registry::handlers::Remove,
        "Remove an existing project entry. Warns when `plan.yaml` references it"
    );

    macro_rules! native {
        ($path:expr, $args:ty, $command:literal, $about:literal) => {
            router = router.route(
                $path,
                run::<$args, Unsupported>()
                    .about($about)
                    .decode_with(unsupported($command))
                    .project_with(SpecifyProjector),
            );
        };
        ($path:expr, $args:ty, $command:literal, $about:literal, $long_about:literal) => {
            router = router.route(
                $path,
                run::<$args, Unsupported>()
                    .about($about)
                    .long_about($long_about)
                    .decode_with(unsupported($command))
                    .project_with(SpecifyProjector),
            );
        };
    }

    native!(
        ["adapters", "sync"],
        adapters::SyncArgs,
        "adapters sync",
        "Hydrate every declared pinned adapter identity into the global store",
        "Hydrate every declared pinned adapter identity into the global store (the explicit hydration trigger).\n\nReads `project.yaml` (the `adapter:` pin plus the `adapters:` prefetch list) and `plan.yaml` source pins when a plan is present, probes the global store per identity, pulls on miss through the wasm-pkg transport, verifies each entry's digest (store sidecar and the committed `.specify/adapters.lock`), and prints the resolved set with per-identity store paths and digests. A warm store makes sync a no-op probe. Bare, unpinned names keep project-local resolution and never hydrate."
    );
    native!(
        ["workspace", "sync"],
        workspace::SyncArgs,
        "workspace sync",
        "Create symlinks or git clones under `workspace/<name>/`. No-op when `registry.yaml` is absent"
    );
    router = router.route(
        ["workspace", "prepare"],
        run::<workspace::PrepareArgs, Unsupported>()
            .about("Hidden executor helper: prepare one workspace slot on `specify/<change>`")
            .hidden()
            .decode_with(unsupported("workspace prepare"))
            .project_with(SpecifyProjector),
    );
    native!(
        ["workspace", "push"],
        workspace::PushArgs,
        "workspace push",
        "Push workspace clones to their remote repositories"
    );
    native!(
        ["upgrade"],
        UpgradeArgs,
        "upgrade",
        "Self-update the `specify` binary across its install channel",
        "Self-update the `specify` binary across its install channel.\n\nBootstrap verb: operates on the binary, not a project, so it never loads project config. `--channel auto` (the default) detects how the binary was installed (`cargo`, Homebrew, or a pre-built release archive); pass `--channel` to override. The target version is the latest GitHub release when reachable, otherwise a HEAD install for the `cargo` channel. `--dry-run` reports the detected channel, the target version, and the exact command(s) that would run without changing anything; applying requires `--yes` (the verb never prompts)."
    );
    native!(
        ["plugins", "doctor"],
        plugins::DoctorArgs,
        "plugins doctor",
        "Report Cursor plugin-cache drift against the marketplace",
        "Report Cursor plugin-cache drift against the marketplace.\n\nRead-only. Resolves the marketplace (`--marketplace`, then `<project-dir>/.cursor-plugin/marketplace.json`, then the XDG config dir), scans `$CURSOR_HOME/plugins/cache/<name>/`, and classifies each declared plugin as `ok | drifted | present | missing`, plus any undeclared cache entry as `extra`. Never exits non-zero on drift — drift is a finding; only filesystem or marketplace-parse failures fail."
    );
    native!(
        ["plugins", "refresh"],
        plugins::RefreshArgs,
        "plugins refresh",
        "Invalidate the Cursor plugin cache for the marketplace",
        "Invalidate the Cursor plugin cache for the marketplace.\n\nDeletes `$CURSOR_HOME/plugins/cache/<name>/`, journals `plugins.refreshed`, and prints a restart instruction. The CLI never restarts Cursor or touches open IDE state. Requires `--yes`; the verb never prompts (consent is the skill's job)."
    );

    for help in NAMESPACE_HELP {
        router = router.namespace(help.path.iter().copied(), help.metadata);
    }
    router.build()
}

fn check_plan_dir(plan_dir: Option<&Path>) -> Result<(), error::Error> {
    let Some(dir) = plan_dir else {
        return Ok(());
    };
    let same = dir == Path::new(".")
        || fs::canonicalize(dir)
            .and_then(|requested| fs::canonicalize(".").map(|root| requested == root))
            .unwrap_or(false);
    if same {
        return Ok(());
    }
    Err(error::Error::Argument {
        flag: "--plan-dir",
        detail: format!(
            "`--plan-dir` must be the project root: plan artifacts anchor at the working \
             directory, so {} would be ignored; run from the plan root instead",
            dir.display()
        ),
    })
}

#[derive(Debug, Serialize)]
struct UnsupportedBody;

impl Render for UnsupportedBody {
    fn render(&self, _writer: &mut dyn std::io::Write) -> std::io::Result<()> {
        Ok(())
    }
}

/// A native-only verb with no guest implementation yet. Its input is
/// uninhabited: the route's decoder always fails, so `call` is never
/// reached.
#[derive(Clone, Copy, Debug)]
struct Unsupported;

impl<P: Provider> Operation<P> for Unsupported {
    type Error = workflow::handler::Error;
    type Input = std::convert::Infallible;
    type Output = UnsupportedBody;

    async fn call(
        input: Self::Input, _context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        match input {}
    }
}

/// Decoder for [`Unsupported`] routes: refuses every parse with the
/// named command's guest-unsupported diagnostic.
fn unsupported<A>(
    command: &'static str,
) -> impl Fn(A, &Globals) -> Result<std::convert::Infallible, error::Error> + Clone + Send + Sync + 'static
{
    move |_args, _globals| {
        Err(error::Error::Argument {
            flag: "<command>",
            detail: format!("`specify {command}` has no guest implementation yet"),
        })
    }
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

convert!(source::ResolveArgs => workflow::adapter::handlers::ResolveInput { value, project_dir });
convert!(target::ResolveArgs => workflow::adapter::handlers::ResolveInput { value, project_dir });
convert!(source::SurveyArgs => workflow::orchestrate::handlers::SurveyInput { source, plan });
convert!(source::ExtractArgs => workflow::orchestrate::handlers::ExtractInput { source, lead, slice });
convert!(slice::CreateArgs => workflow::slice::handlers::CreateInput { name, target, if_exists });
convert!(slice::ValidateArgs => workflow::slice::handlers::ValidateInput { name });
convert!(slice::ProvenanceArgs => workflow::slice::handlers::ProvenanceInput { name });
convert!(slice::ModelShowArgs => workflow::slice::handlers::ModelShowInput { name });
convert!(slice::RefineArgs => workflow::orchestrate::handlers::RefineInput { name });
convert!(slice::BuildArgs => workflow::orchestrate::handlers::BuildInput { name });
convert!(slice::MergeRunArgs => workflow::orchestrate::handlers::MergeRunInput { name, allow_composition_replace });
convert!(slice::MergePreviewArgs => workflow::slice::handlers::PreviewInput { name });
convert!(slice::ConflictCheckArgs => workflow::slice::handlers::ConflictCheckInput { name });
convert!(slice::TaskProgressArgs => workflow::slice::handlers::TaskProgressInput { name });
convert!(slice::TaskMarkArgs => workflow::slice::handlers::TaskMarkInput { name, task_number });
convert!(slice::TransitionArgs => workflow::slice::handlers::TransitionInput { name, target });
convert!(slice::TouchedSpecsArgs => workflow::slice::handlers::TouchedSpecsInput { name, scan, set });
convert!(slice::OverlapArgs => workflow::slice::handlers::OverlapInput { name });
convert!(slice::DropArgs => workflow::slice::handlers::DropInput { name, reason });
convert!(archive::PruneArgs => workflow::slice::handlers::PruneInput { keep, older_than, dry_run });
convert!(plan::CreateArgs => workflow::change::plan::handlers::CreateInput { name, sources, intent, auto_approve, authority_override });
convert!(plan::ValidateArgs => workflow::change::plan::handlers::ValidateInput {});
convert!(plan::NextArgs => workflow::change::plan::handlers::NextInput {});
convert!(plan::StatusArgs => workflow::change::plan::handlers::StatusInput {});
convert!(plan::ExecuteArgs => workflow::orchestrate::handlers::ExecuteInput {});
convert!(plan::AddArgs => workflow::change::plan::handlers::AddInput { name, depends_on, sources, description, project, context, authority_override });
convert!(plan::AmendArgs => workflow::change::plan::handlers::AmendInput { name, depends_on, sources, add_source, remove_source, divergence, description, project, context, authority_override, clear_authority_override, clear_authority_overrides });
convert!(plan::RemoveArgs => workflow::change::plan::handlers::RemoveInput { name });
convert!(plan::TransitionArgs => workflow::change::plan::handlers::TransitionInput { name, target, undo, actor });
convert!(plan::AuthorArgs => workflow::orchestrate::handlers::AuthorInput { name, sources, intent });
convert!(plan::ArchiveArgs => workflow::change::plan::handlers::ArchiveInput { force });
convert!(journal::EmitArgs => workflow::journal::handlers::EmitInput { event, payload });
convert!(journal::ShowArgs => workflow::journal::handlers::ShowInput { filter, limit });
convert!(registry::ValidateArgs => workflow::registry::handlers::ValidateInput {});
convert!(registry::AddArgs => workflow::registry::handlers::AddInput { name, url, adapter, description });
convert!(registry::RemoveArgs => workflow::registry::handlers::RemoveInput { name });

impl TryFrom<InitArgs> for workflow::init::handlers::ScaffoldInput {
    type Error = error::Error;

    fn try_from(args: InitArgs) -> Result<Self, Self::Error> {
        if !args.scaffold_only {
            return Err(error::Error::Argument {
                flag: "<command>",
                detail: "`specify init` has no guest implementation yet".to_string(),
            });
        }
        Ok(Self {
            adapter: args.adapter,
            name: args.name,
            description: args.description,
            workspace: args.workspace,
            platforms: args.platforms,
        })
    }
}

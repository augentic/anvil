//! The exhaustive route inventory: the typed clap grammar, namespace
//! help, the router assembly, and the `Args`-to-`Input` conversions.

use clap::Args;
use omnia_guest::Model;
use omnia_guest::api::Provider;
use omnia_guest::api::command::{BuildError, Completions, Namespace, Router, RouterBuilder, run};
use omnia_guest::api::invoke::Invoker;
use project::adapter::Resolver;
use project::handler::Anchor;
use project::seam::{Source, Target, Workspaces};

use super::{
    EmeryProjector, Globals, adapter, archive, journal, plan, registry, slice, source, target,
};

/// One-line application description.
const ABOUT: &str = "Deterministic primitives for spec-driven development";

/// Flags for `emery init`.
#[derive(Debug, Args)]
pub(super) struct InitArgs {
    /// Adapter identifier or local component path.
    #[arg(conflicts_with = "workspace")]
    pub(super) adapter: Option<String>,
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
    /// Re-enter initialization to bump the Emery version pin.
    #[arg(long, conflicts_with_all = ["adapter", "workspace", "name", "description"])]
    pub(super) upgrade: bool,
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
        "Adapter component operations. `add` seeds a local `.wasm` component into the project component cache — pre-init, axis-neutral — so bare bindings (project target, plan sources) resolve it; `upgrade` refreshes a bare name (or every bare project binding with `--all`) to the newest published version (the explicit upgrade act — normal resolution is local-first and never checks the registry while something local exists)",
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
    NamespaceHelp::new(
        &["archive"],
        "Slice-archive cache maintenance. The archived slice folders under `.emery/archive/` are a prunable convenience cache; `prune` reclaims disk by retention bound",
    ),
    NamespaceHelp::new(&["plan"], "Executable plan operations — `plan.yaml` lifecycle"),
    NamespaceHelp::new(
        &["journal"],
        "Workflow journal at `.emery/journal.jsonl`. Read-only: `show` projects the closed §Observability event taxonomy; CLI verbs append their own events as a side effect of the operation",
    ),
    NamespaceHelp::new(&["registry"], "Platform registry at `registry.yaml` (repo root)"),
];

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
    P: Provider + Anchor + Model + Resolver + Source + Target + Workspaces,
{
    let command = clap::Command::new("emery").version(env!("CARGO_PKG_VERSION")).about(ABOUT);
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
        ["adapter", "upgrade"],
        adapter::UpgradeArgs,
        project::adapter::handlers::AdapterUpgrade,
        "Upgrade a bare-named adapter (or every bare project binding with `--all`) to the newest published version",
        "Upgrade a bare-named adapter (or every bare project binding with `--all`) to the newest published version.\n\nThe explicit upgrade act: normal resolution is local-first (project cache seed, else the newest installed store version) and never checks the registry while something local exists. This verb forces the registry check — the deployment lists the published exact-SemVer versions, installs the newest into the global store, and the verb reports what each name now resolves to. `--all` collects every bare binding the project records (`project.yaml` target plus `plan.yaml` sources). A project cache seed always wins and is reported as such; refresh it by re-running `emery adapter add`. Pinned versions (`emery:<name>@<semver>`) are immutable and are refused here."
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
        "Debug/breakout: re-run one source adapter's `survey` against a plan-bound source and merge the resulting lead set into `discovery.md` — `plan author` runs this step itself",
        "Debug/breakout: re-run one source adapter's `survey` against a plan-bound source and merge the resulting lead set into `discovery.md` — `plan author` runs this step itself; reach for this verb only to re-survey a single source or debug adapter wiring.\n\nResolves `<source>` against `plan.yaml.sources.<key>` (not the adapter name) and drives the bound source adapter's collapsed survey orchestration in the engine guest — one call covering the source dispatch, `leads.md` validation, and the `discovery.md` merge."
    );
    route!(
        ["source", "extract"],
        source::ExtractArgs,
        ::slice::source::Extract,
        "Debug/breakout: re-run one source adapter's `extract` for one `(source, lead)` pair and persist the resulting Evidence to `.emery/slices/<slice>/evidence/<source>.yaml` — `slice refine` runs this step itself",
        "Debug/breakout: re-run one source adapter's `extract` for one `(source, lead)` pair and persist the resulting Evidence to `.emery/slices/<slice>/evidence/<source>.yaml` — `slice refine` runs this step itself; reach for this verb only to re-extract a single source or debug adapter wiring.\n\nResolves `<source>` against `plan.yaml.sources.<key>` (not the adapter name) and drives the bound source adapter's collapsed extract orchestration in the engine guest — one call covering the source dispatch, the typed Evidence validation, and the persist."
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
        ["slice", "merge"],
        slice::MergeArgs,
        ::slice::handlers::MergeRun,
        "Merge all delta specs for the slice into baseline and archive the slice",
        "Merge all delta specs for the slice into baseline and archive the slice.\n\n`--preview` shows the merge operations that would be applied and `--conflict-check` reports `type: modified` baselines modified after this slice's `defined_at` — both are read-only dry-run modes that write nothing. Re-entry heals a torn merge: when the commit already landed but the per-entry `done` stamp is missing, the run stamps the entry and returns without a second baseline merge."
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
        ["plan", "advance"],
        plan::AdvanceArgs,
        ::change::plan::handlers::Advance,
        "Advance the next eligible `pending` entry to `in-progress` and return it, or return the already-active entry unchanged. `plan advance` writes plan state — it is the only writer of per-entry `in-progress` (workflow §CLI surface); use `plan status` for the read-only projection"
    );
    route!(
        ["plan", "status"],
        plan::StatusArgs,
        ::change::plan::handlers::Status,
        "Read-only projection of the plan's execution state into a deterministic `next-action` — `refine|build|merge <slice>`, `stop <reason>`, or `drained`",
        "Read-only projection of the plan's execution state into a deterministic `next-action` — `refine|build|merge <slice>`, `stop <reason>`, or `drained`.\n\nProjects `plan.yaml` entries, the candidate slice's `metadata.yaml` lifecycle (slot-aware in workspace mode), and the journal tail. Stop reasons (`refine-failed`, `build-failed`, `merge-conflict`, `merge-postflight-failed`, `slice-dropped`, `merge-incomplete`, `stuck`) are classified from `slice.synthesize.failed` / `slice.build.failed` / `slice.merge.failed` / `slice.merge.postflight-failed` journal events (scoped to the active entry's window for in-progress failures; plan-scoped sticky debt for postflight until `plan.merge-postflight.acknowledged`). Writes nothing — `plan advance` stays the only writer of per-entry `in-progress`."
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
        "Remove a pending plan entry while the plan is still replaceable (every entry `pending`). Plan-review curation only — defers a lead without re-surveying `discovery.md`"
    );
    route!(
        ["plan", "undo"],
        plan::UndoArgs,
        ::change::plan::handlers::Undo,
        "Walk one plan entry backwards on per-entry status (one rung, or `--to <status>`)",
        "Walk one plan entry backwards on per-entry status.\n\n`<name>` is a plan-entry name. Legal rungs: `done → in-progress`, `in-progress → pending`. Default is one rung; `--to <pending|in-progress>` walks rung by rung until the entry reaches the target. Either way, one `plan.transition.undone` journal event fires per rung, so the journal records every step.\n\nPer-entry `pending` is written only by `plan add` / `plan amend`; per-entry `in-progress` is written only by `plan advance`; per-entry `done` is written only by `slice merge`. v1 has no per-entry `blocked`, `failed`, or `skipped` state — build failures and merge conflicts leave the active entry `in-progress`."
    );
    route!(
        ["plan", "author"],
        plan::AuthorArgs,
        ::change::plan::handlers::Author,
        "Author a plan end-to-end in the engine guest: scaffold `plan.yaml`, survey every bound source into `discovery.md`, reconcile the leads into `plan.yaml.slices[]` through the judgment leg, persist the review prose (`change.md`, `discovery.md`'s `## Summary` and `## Source inventory`), validate, and exit with the literal execute hint",
        "Author a plan end-to-end in the engine guest: scaffold `plan.yaml`, survey every bound source into `discovery.md`, reconcile the leads into `plan.yaml.slices[]` through the judgment leg, persist the review prose (`change.md`, `discovery.md`'s `## Summary` and `## Source inventory`), validate, and exit with the literal execute hint.\n\nAn existing `plan.yaml` refuses with `plan-already-exists` unless `--force` is set; `--force` recreates the plan unconditionally, whatever its entry statuses. Guest-only through the composed-deployment leg: the `/emery:plan` skill invokes this single verb and relays its output."
    );
    route!(
        ["plan", "execute"],
        plan::ExecuteArgs,
        ::change::plan::handlers::Execute,
        "Run the drained execute loop in the engine guest: advance → refine → build → merge per entry until the plan projects `drained` or a stop condition halts it (exit 2, `plan-execute-stopped`). Running execute on an authored plan is the approval — there is no recorded approval state",
        "Run the drained execute loop in the engine guest: advance → refine → build → merge per entry until the plan projects `drained` or a stop condition halts it (exit 2, `plan-execute-stopped`).\n\nRunning execute on an authored plan is the approval — nothing is stamped or recorded. Guest-only through the composed-deployment leg: the loop holds the create-exclusive `.emery/guest.lock` marker (guest-vs-guest refusal only) while it drives the phases."
    );
    route!(
        ["plan", "archive"],
        plan::ArchiveArgs,
        ::change::plan::handlers::Archive,
        "Archive the current plan to `.emery/archive/plans/<name>-<YYYYMMDD>.yaml`"
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
convert!(adapter::UpgradeArgs => project::adapter::handlers::UpgradeInput { name, all, project_dir });
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
convert!(slice::MergeArgs => ::slice::handlers::MergeRunInput { name, allow_composition_replace, preview, conflict_check });
convert!(slice::DropArgs => ::slice::handlers::DropInput { name, reason });
convert!(archive::PruneArgs => ::slice::handlers::PruneInput { keep, older_than, dry_run });
convert!(plan::ValidateArgs => ::change::plan::handlers::ValidateInput {});
convert!(plan::AdvanceArgs => ::change::plan::handlers::AdvanceInput {});
convert!(plan::StatusArgs => ::change::plan::handlers::StatusInput {});
convert!(plan::ExecuteArgs => ::change::plan::handlers::ExecuteInput {});
convert!(plan::AddArgs => ::change::plan::handlers::AddInput { name, depends_on, sources, description, project, context, authority_override });
convert!(plan::AmendArgs => ::change::plan::handlers::AmendInput { name, depends_on, sources, add_source, remove_source, divergence, description, project, context, authority_override, clear_authority_override, clear_authority_overrides });
convert!(plan::RemoveArgs => ::change::plan::handlers::RemoveInput { name });
convert!(plan::UndoArgs => ::change::plan::handlers::UndoInput { name, to });
convert!(plan::AuthorArgs => ::change::plan::handlers::AuthorInput { name, sources, intent, force });
convert!(plan::ArchiveArgs => ::change::plan::handlers::ArchiveInput { force });
convert!(journal::ShowArgs => project::journal::handlers::ShowInput { filter, limit });
convert!(registry::ValidateArgs => project::registry::handlers::ValidateInput {});
convert!(registry::AddArgs => project::registry::handlers::AddInput { name, url, adapter, description });
convert!(registry::RemoveArgs => project::registry::handlers::RemoveInput { name });

convert!(InitArgs => project::init::handlers::InitInput { adapter, name, description, workspace, platforms, upgrade });

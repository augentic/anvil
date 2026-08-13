//! The exhaustive route inventory: the typed clap grammar, namespace
//! help, the router assembly, and the `Args`-to-`Input` conversions.

use clap::Args;
use omnia_guest::Model;
use omnia_guest::api::Provider;
use omnia_guest::api::command::{BuildError, Completions, Namespace, Router, RouterBuilder, run};
use omnia_guest::api::invoke::Invoker;
use project::adapter::Resolver;
use project::handler::Anchor;
use project::seam::{Origins, Source, Target, Workspaces};

use super::{
    EmeryProjector, Globals, adapter, archive, journal, plan, slice, source, system, target,
};

/// One-line application description.
const ABOUT: &str = "Deterministic primitives for spec-driven development";

/// Flags for `emery init`.
#[derive(Debug, Args)]
pub(super) struct InitArgs {
    /// Adapter identifier or local component path.
    pub(super) adapter: Option<String>,
    /// Project name.
    #[arg(long)]
    name: Option<String>,
    /// Project description.
    #[arg(long)]
    description: Option<String>,
    /// Comma-separated target platforms.
    #[arg(long)]
    platforms: Option<String>,
    /// Re-enter initialization to bump the Emery version pin.
    #[arg(long, conflicts_with_all = ["adapter", "name", "description"])]
    pub(super) upgrade: bool,
}

/// Arguments for `debt` — none.
#[derive(Clone, Copy, Debug, Args)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "clap's `Args` derive requires a braced struct"
)]
pub(super) struct DebtArgs {}

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
        "Source adapter operations (workflow contract) — debug/breakout surface; `plan author` and `plan execute` run these steps themselves. Source adapters provide `extract` + `survey` capabilities and resolve to a single `.wasm` component: the global store entry for pinned identities, the seeded project component cache for bare names",
    ),
    NamespaceHelp::new(
        &["target"],
        "Target adapter operations (workflow contract) — debug/breakout surface; the execute loop's build and merge phases resolve targets themselves. Target adapters provide `guidance` + `build` + `merge` capabilities and resolve to a single `.wasm` component: the global store entry for pinned identities, the seeded project component cache for bare names",
    ),
    NamespaceHelp::new(
        &["slice"],
        "Read-only slice projections over `.emery/slices/` — `plan refine` owns refinement; the execute loop owns the build → merge phases",
    ),
    NamespaceHelp::new(&["slice", "model"], "Read-only viewer over a slice's `model.yaml`"),
    NamespaceHelp::new(
        &["archive"],
        "Slice-archive cache maintenance. The archived slice folders under `.emery/archive/` are a prunable convenience cache; `prune` reclaims disk by retention bound",
    ),
    NamespaceHelp::new(&["plan"], "Executable plan operations — `plan.yaml` lifecycle"),
    NamespaceHelp::new(
        &["journal"],
        "Workflow journal at `.emery/events/<writer>.jsonl`. Read-only: `show` merges the per-writer union and projects the closed §Observability event taxonomy; CLI verbs append their own events as a side effect of the operation",
    ),
    NamespaceHelp::new(
        &["system"],
        "Definition-loop operations over a definition home (RFC-104) — a durable root the operator authors by hand (`scope.yaml` + `coverage.yaml`), never a product checkout. The launcher mounts `--dir`-or-CWD as the invocation's root with no `project.yaml` walk",
    ),
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
    P: Provider + Anchor + Model + Resolver + Source + Target + Workspaces + Origins,
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
        "Initialize .emery/ in a project.\n\nPass `<adapter>` (first-party shorthand, package reference, or local component path). A missing `<adapter>` fails typed with `init-adapter-required` (exit 2). Re-running `init` in an already-initialized project changes nothing and exits 0 routing to `emery init --upgrade`."
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
        "Debug/breakout: re-run one source adapter's `extract` for one `(source, lead)` pair and persist the resulting Evidence to `.emery/slices/<slice>/evidence/<source>.yaml` — the `plan refine` drain runs this step itself",
        "Debug/breakout: re-run one source adapter's `extract` for one `(source, lead)` pair and persist the resulting Evidence to `.emery/slices/<slice>/evidence/<source>.yaml` — the `plan refine` drain runs this step itself; reach for this verb only to re-extract a single source or debug adapter wiring.\n\nResolves `<source>` against `plan.yaml.sources.<key>` (not the adapter name) and drives the bound source adapter's collapsed extract orchestration in the engine guest — one call covering the source dispatch, the typed Evidence validation, and the persist."
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
        "Validate a slice's artifacts against adapter validation rules",
        "Validate a slice's artifacts against adapter validation rules.\n\nBlocking findings gate the execute loop; non-blocking review advisories include baseline-drift signals — `type: modified` baselines modified after this slice's `defined_at` (the retired `slice merge --conflict-check` probe)."
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
        "Validate plan.yaml (structure + plan/change consistency).\n\nIncludes the health diagnostics — `cycle-in-depends-on` and `orphan-source` — alongside the base shape rules."
    );
    route!(
        ["plan", "status"],
        plan::StatusArgs,
        ::change::plan::handlers::Status,
        "Read-only projection of the plan's execution state into a deterministic `next-action` — `refine|build|merge <slice>`, `stop <reason>`, or `drained`",
        "Read-only projection of the plan's execution state into a deterministic `next-action` — `refine|build|merge <slice>`, `stop <reason>`, or `drained` — plus Ready / Authorized milestones (never `approved`).\n\nComputed from `plan.yaml` topology, slice artifacts / phase timestamps, and the per-writer fact union. Stop reasons (`refine-failed`, `build-failed`, `merge-conflict`, `merge-postflight-failed`, `slice-dropped`, `merge-incomplete`, `stuck`) are classified from phase / wave journal events (scoped to the active entry's window for in-progress failures; plan-scoped sticky debt for postflight until `plan.merge-postflight.acknowledged`). Writes nothing."
    );
    route!(
        ["plan", "gaps"],
        plan::GapsArgs,
        ::change::plan::handlers::Gaps,
        "Read-only typed gap inventory across in-scope slices (`unknown` / `conflict` / `divergence`) with shared-lead re-refine suggestions",
        "Read-only typed gap inventory across in-scope slices (`unknown` / `conflict` / `divergence`) with shared-lead re-refine suggestions.\n\nDerived from on-disk `model.yaml` / `specs/<domain>/spec.md` — not a second file to keep in sync. Dropped slices are excluded. Shared-lead rollup is presentation only; deferral dispositions and the execute gap gate stay per-requirement."
    );
    route!(["plan", "add"], plan::AddArgs, ::change::plan::handlers::Add, "Add a new plan entry");
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
        "Remove a plan entry while the plan is still replaceable (every entry still projects `pending`). Plan-review curation only — defers a lead without re-surveying `discovery.md`"
    );
    route!(
        ["plan", "drop"],
        plan::DropArgs,
        ::change::plan::handlers::Drop,
        "Abandon one plan entry's slice without merging: stamp it `dropped` and archive the slice tree",
        "Abandon one plan entry's slice without merging: stamp `dropped_at` on the slice's `metadata.yaml` and archive the slice tree under `.emery/archive/`.\n\nDropped slices leave the in-scope set (`plan gaps` excludes them) and the entry stays on the plan for the record; `plan status` projects the `slice-dropped` stop for it. A never-refined entry has no slice tree — curate it with `emery plan remove` instead."
    );
    route!(
        ["plan", "author"],
        plan::AuthorArgs,
        ::change::plan::handlers::Author,
        "Author a plan end-to-end in the engine guest: scaffold `plan.yaml`, survey every bound source into `discovery.md`, reconcile the leads into `plan.yaml.slices[]` through the judgment leg, persist the review prose (`change.md`, `discovery.md`'s `## Summary` and `## Source inventory`), validate, and exit with the literal execute hint",
        "Author a plan end-to-end in the engine guest: scaffold `plan.yaml`, survey every bound source into `discovery.md`, reconcile the leads into `plan.yaml.slices[]` through the judgment leg, persist the review prose (`change.md`, `discovery.md`'s `## Summary` and `## Source inventory`), validate, and exit with the literal execute hint.\n\nWhen the baseline carries deferred debt, `change.md` also renders the carried-debt inventory (the same backlog `emery debt` projects), so a corrective change is scoped with it in view. An existing `plan.yaml` refuses with `plan-already-exists` unless `--force` is set; `--force` recreates the plan unconditionally, whatever its entry statuses. Guest-only through the composed-deployment leg: the `/emery:plan` skill invokes this single verb and relays its output."
    );
    route!(
        ["plan", "refine"],
        plan::RefineArgs,
        ::change::plan::handlers::Refine,
        "Drain refinement for a closed plan: extract + synthesize every targeted in-scope leaf in dependency order, write per-slice refinement manifests, and stop before any code work",
        "Drain refinement for a closed plan in the engine guest — the specification stage between `plan author` and `plan execute` (RFC-91).\n\nWalks in-scope plan entries in topological `depends-on` order and, for every targeted leaf whose refinement manifest is missing or stale, extracts each bound source, synthesizes and validates the slice artifacts, and atomically writes `refinement.yaml`. Fresh leaves are skipped, so re-running resumes missing or stale work; the drain stops on the first failed refinement (exit 2, `plan-refine-stopped`).\n\nRepeatable `--slice <name>` targets specific leaves plus the stale-or-missing predecessor closure they need. Successful refinement may carry `[unknown]` / `[conflict]` / `[divergence]` review outputs — inspect them with `emery plan gaps`. No target build operation, workspace, wave, or authorization epoch is created; the loop holds the create-exclusive `.emery/guest.lock` marker while it drains."
    );
    route!(
        ["plan", "execute"],
        plan::ExecuteArgs,
        ::change::plan::handlers::Execute,
        "Run the drained execute loop: at start append `plan.execute.started` (authorization epoch) covering exact per-leaf refinement digests, then build → merge until `drained` or a stop (exit 2, `plan-execute-stopped`)",
        "Run the drained execute loop in the engine guest.\n\nRequires a fresh refinement manifest for every in-scope leaf it may build — execute never refines (RFC-91): a missing or stale manifest is a typed `refinement-required` stop pointing at `emery plan refine`. At start appends `plan.execute.started` with typed `closed-plan` coverage — exact per-leaf refinement digests. Then advance → build → merge per entry until the plan projects `drained` or a stop condition halts it (exit 2, `plan-execute-stopped`).\n\nBefore each build the gap gate joins durable dispositions from the deferral fact union: deferred rows leave build scope; open `[unknown]` / `[conflict]` rows are dispositioned at the gate (one `gap.deferred` fact each) and build proceeds — nothing blocks. Guest-only through the composed-deployment leg: the loop holds the create-exclusive `.emery/guest.lock` marker (guest-vs-guest refusal only) while it drives the phases."
    );
    route!(
        ["plan", "archive"],
        plan::ArchiveArgs,
        ::change::plan::handlers::Archive,
        "Archive the current plan to `.emery/archive/plans/<name>-<YYYYMMDD>.yaml` and sweep the snapshot objects whose GC roots belonged only to the archived change"
    );
    route!(
        ["debt"],
        DebtArgs,
        ::slice::handlers::Debt,
        "Read-only baseline debt projection: list every carried `unknown` / `conflict` requirement under `.emery/specs/` with the reason, originating change, and age from its deferral note",
        "Read-only baseline debt projection (RFC-86a D9) — the backlog looking ahead.\n\nWalks the baseline specs under `.emery/specs/` and lists every requirement whose status is `unknown` or `conflict`, with the reason, originating change, and age parsed from the self-describing deferral note the merge fold appended. Conflicts render separately from unknowns. Reads the baseline alone — never archived fact logs — and writes nothing. `plan author` renders the same inventory in the review prose it authors, so a corrective change is scoped with the backlog in view."
    );
    route!(
        ["system", "survey"],
        system::SurveyArgs,
        ::system::handlers::Survey,
        "Survey the declared coverage of a definition home",
        "Survey the declared coverage of a definition home (RFC-104).\n\nAnchors at `--dir` (or the current directory) with no `project.yaml` walk — a definition home is durable client architecture, not a product checkout. Fails closed when `scope.yaml` or `coverage.yaml` is missing; the operator authors both by hand (there is no `system init`)."
    );
    route!(
        ["journal", "show"],
        journal::ShowArgs,
        project::journal::handlers::Show,
        "Read events from `.emery/events/<writer>.jsonl` (union order)",
        "Read events from `.emery/events/<writer>.jsonl`, merging every writer file in `(timestamp, writer, sequence)` order.\n\nRead-only: emits no journal event and writes nothing. Text mode prints the canonical JSONL lines — one `{ timestamp, writer, sequence, event, payload }` object per event, pipeable — while `--format json` wraps the same events in the standard envelope. Blank and unparseable lines are skipped, matching every other journal reader; a missing events directory yields no events."
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
convert!(archive::PruneArgs => ::slice::handlers::PruneInput { keep, older_than, dry_run });
convert!(plan::ValidateArgs => ::change::plan::handlers::ValidateInput {});
convert!(plan::StatusArgs => ::change::plan::handlers::StatusInput {});
convert!(plan::GapsArgs => ::change::plan::handlers::GapsInput {});
convert!(plan::RefineArgs => ::change::plan::handlers::RefineInput { slice });
convert!(plan::ExecuteArgs => ::change::plan::handlers::ExecuteInput {});
convert!(plan::AddArgs => ::change::plan::handlers::AddInput { name, depends_on, sources, description, context, authority_override });
convert!(plan::AmendArgs => ::change::plan::handlers::AmendInput { name, depends_on, sources, add_source, remove_source, divergence, description, context, authority_override, clear_authority_override, clear_authority_overrides, allow_composition_replace });
convert!(plan::RemoveArgs => ::change::plan::handlers::RemoveInput { name });
convert!(plan::DropArgs => ::change::plan::handlers::DropInput { name, reason });
convert!(plan::AuthorArgs => ::change::plan::handlers::AuthorInput { name, sources, intent, force });
convert!(plan::ArchiveArgs => ::change::plan::handlers::ArchiveInput { force });
convert!(journal::ShowArgs => project::journal::handlers::ShowInput { filter, limit });
convert!(DebtArgs => ::slice::handlers::DebtInput {});

// Deliberately not `convert!`: `--dir` is deployment-consumed — the
// launcher anchors the `.` mount at it (`selectors::system_request`),
// so the operation reads the anchored root.
impl TryFrom<system::SurveyArgs> for ::system::handlers::SurveyInput {
    type Error = error::Error;

    fn try_from(args: system::SurveyArgs) -> Result<Self, Self::Error> {
        let system::SurveyArgs { dir } = args;
        drop(dir);
        Ok(Self {})
    }
}

convert!(InitArgs => project::init::handlers::InitInput { adapter, name, description, platforms, upgrade });

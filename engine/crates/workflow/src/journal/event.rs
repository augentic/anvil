//! Closed journal event taxonomy and wire DTOs.
//!
//! Wire format is locked: event ids are dotted kebab-case
//! (`plan.transition.approved`), payload field names are kebab-case
//! (`plan-name`, `slice-name`, …), and the closed `from` / `to`
//! enum is `none | likely | accepted | rejected`. Rust variant
//! names stay `snake_case` and reach the wire through
//! `#[serde(rename = "…")]`.

use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::adapter::operation::SourceOperation;
use crate::change::Divergence;
use crate::name::{PlanName, SliceName};

/// One row of the journal. Serialises as `{ timestamp, event,
/// payload }` — workflow §Wire format pins `timestamp` first so a
/// `head -1` on the file is enough to confirm the run window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Second-precision UTC timestamp (`%Y-%m-%dT%H:%M:%SZ`).
    #[serde(with = "specify_error::serde_rfc3339")]
    pub timestamp: Timestamp,
    /// Event id + payload, adjacently tagged so `event` and `payload`
    /// sit side by side in the JSON object.
    #[serde(flatten)]
    pub kind: EventKind,
}

impl Event {
    /// Build an [`Event`] at `timestamp` carrying `kind`. Tests pin
    /// the timestamp; production callers pass `Timestamp::now()`.
    #[must_use]
    pub const fn new(timestamp: Timestamp, kind: EventKind) -> Self {
        Self { timestamp, kind }
    }
}

/// The workflow §Observability event set.
///
/// Adjacently-tagged on the wire as `{ event: <id>, payload: {…} }`
/// so the dotted-kebab-case event id is a top-level field consumers
/// can filter on without parsing the payload first.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", content = "payload")]
pub enum EventKind {
    /// Gate 1 cleared — `specify plan transition <plan-name> approved`.
    #[serde(rename = "plan.transition.approved", rename_all = "kebab-case")]
    PlanTransitionApproved {
        /// Plan name from `plan.yaml.name`.
        plan_name: PlanName,
        /// Who drove the stamp. Self-reported via `--actor`
        /// (default `operator`); evidence for eval probes, never
        /// enforcement. `#[serde(default)]` keeps pre-actor journal
        /// lines parseable as `operator`.
        #[serde(default)]
        actor: Actor,
    },
    /// Operator walked one rung backwards on per-entry status via
    /// `specify plan transition <entry> --undo`. One event per rung
    /// (`done → in-progress` and `in-progress → pending` each fire
    /// individually) so the journal records every step the operator
    /// took and replay traces line up with the forward-direction
    /// `plan.transition.approved` / `slice.transition.*` cadence.
    #[serde(rename = "plan.transition.undone", rename_all = "kebab-case")]
    PlanTransitionUndone {
        /// Plan name from `plan.yaml.name`.
        plan_name: PlanName,
        /// Entry id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// Status the entry held before the undo.
        from: crate::change::Status,
        /// Status the entry holds after the undo.
        to: crate::change::Status,
    },
    /// `specify plan next` advanced one entry `pending → in-progress`
    /// (the sole writer of per-entry `in-progress`). Fires only when
    /// an entry actually advanced — returning the already-active entry
    /// or reporting drained/stuck emits nothing, so the *absence* of
    /// this event over a window is probeable evidence that the execute
    /// loop parked rather than advancing.
    #[serde(rename = "plan.entry.advanced", rename_all = "kebab-case")]
    PlanEntryAdvanced {
        /// Plan name from `plan.yaml.name`.
        plan_name: PlanName,
        /// Entry id under `plan.yaml.slices[].name` that advanced.
        slice_name: SliceName,
    },
    /// Stamped `slices[].divergence` via
    /// `specify plan amend --divergence <likely|accepted|rejected>`.
    /// The CLI is the single writer. In the propose flow the
    /// `/spec:plan` agent stages `likely`
    /// through this event after `propose --from`; the operator later
    /// flips `accepted` / `rejected` the same way. This is the only
    /// path that writes the `divergence` field.
    #[serde(rename = "plan.amend.divergence", rename_all = "kebab-case")]
    PlanAmendDivergence {
        /// Plan name from `plan.yaml.name`.
        plan_name: PlanName,
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// Previous value — may be any of `none | likely | accepted | rejected`.
        /// Callers convert an absent on-disk slice field via
        /// `previous.unwrap_or(Divergence::None)`.
        from: Divergence,
        /// New value — `likely`, `accepted`, or `rejected`. The
        /// implicit `none` default is rejected at the flag-parser
        /// level; omit `--divergence` to leave the field unchanged.
        to: Divergence,
    },
    /// Slice transitioned to `refined` — synthesis finished and the
    /// slice is ready for `/spec:build`.
    #[serde(rename = "slice.transition.refined", rename_all = "kebab-case")]
    SliceTransitionRefined {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
    },
    /// `specify source extract --phase finalize` validated and
    /// persisted one source-bound Evidence document. One event per
    /// `(source, slice)` pair. CLI-owned — the `/spec:refine` skill
    /// never emits this via `specify journal emit`.
    #[serde(rename = "slice.extract.completed", rename_all = "kebab-case")]
    SliceExtractCompleted {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// Source key from `plan.yaml.sources.<key>`.
        source: String,
    },
    /// `[conflict]` on a requirement in `spec.md` — same-authority
    /// disagreement the operator must reconcile. Emitted by
    /// `specify slice validate` after a successful run.
    #[serde(rename = "slice.synthesis.conflict", rename_all = "kebab-case")]
    SliceSynthesisConflict {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// `ID:` value on the tagged requirement block.
        requirement_id: String,
    },
    /// `[divergence]` on a requirement in `spec.md` — cross-authority
    /// disagreement preserved as inline commentary. Emitted by
    /// `specify slice validate` after a successful run.
    #[serde(rename = "slice.synthesis.divergence", rename_all = "kebab-case")]
    SliceSynthesisDivergence {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// `ID:` value on the tagged requirement block.
        requirement_id: String,
    },
    /// `[unknown]` on a requirement in `spec.md` — a gap the operator
    /// must close before the requirement is meaningful. Emitted by
    /// `specify slice validate` after a successful run.
    #[serde(rename = "slice.synthesis.unknown", rename_all = "kebab-case")]
    SliceSynthesisUnknown {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// `ID:` value on the tagged requirement block.
        requirement_id: String,
    },
    /// Slice synthesis began — `/spec:refine` started folding the
    /// extracted evidence into `proposal.md` / `spec.md` / `design.md`
    /// / `tasks.md` / `model.yaml`. One event per slice. Distinct from the per-requirement
    /// `slice.synthesis.*` tag events above — `synthesize` is the
    /// lifecycle verb, `synthesis` is the requirement-tag noun.
    #[serde(rename = "slice.synthesize.started", rename_all = "kebab-case")]
    SliceSynthesizeStarted {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
    },
    /// Synthesis dispatched to the agent. Synthesis is always
    /// agent-driven; this signal fires on the dry-run inputs phase so
    /// the journal records the handoff.
    #[serde(rename = "slice.synthesize.agent", rename_all = "kebab-case")]
    SliceSynthesizeAgent {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
    },
    /// Slice synthesis finished and the artifacts were persisted.
    /// `artifacts` lists the persisted
    /// relative paths (`proposal.md`, `specs/<domain>/spec.md`,
    /// `design.md`, `tasks.md`, `model.yaml`).
    #[serde(rename = "slice.synthesize.completed", rename_all = "kebab-case")]
    SliceSynthesizeCompleted {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// Persisted artifact relative paths, in write order.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        artifacts: Vec<String>,
    },
    /// Slice synthesis failed before all artifacts were persisted.
    /// `reason` carries a short human
    /// reason or finding code so the journal records why the slice
    /// stalled.
    #[serde(rename = "slice.synthesize.failed", rename_all = "kebab-case")]
    SliceSynthesizeFailed {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// Short human reason / finding code for the failure.
        reason: String,
    },
    /// `/spec:build` started implementing the slice — the target
    /// adapter's `build` brief began running against the refined
    /// artifacts. One event per slice.
    #[serde(rename = "slice.build.started", rename_all = "kebab-case")]
    SliceBuildStarted {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
    },
    /// `/spec:build` finished implementing the slice — the target
    /// adapter's `build` brief completed and the slice is ready for
    /// `/spec:merge`. One event per slice.
    #[serde(rename = "slice.build.succeeded", rename_all = "kebab-case")]
    SliceBuildSucceeded {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
    },
    /// `/spec:build` stopped before the slice was implemented.
    /// `reason` carries a short human
    /// reason or finding code so the journal records why the build
    /// stalled.
    #[serde(rename = "slice.build.failed", rename_all = "kebab-case")]
    SliceBuildFailed {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// Short human reason / finding code for the failure.
        reason: String,
    },
    /// `specify slice merge` began folding the slice's deltas into the
    /// baseline. The `slice.merge.*` pair
    /// fires on the `specify slice merge` validator outcome, not on a
    /// merge report. One event per slice.
    #[serde(rename = "slice.merge.started", rename_all = "kebab-case")]
    SliceMergeStarted {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
    },
    /// `specify slice merge` validated and applied the slice's deltas
    /// to the baseline. Fires on the
    /// validator outcome, not on a merge report. One event per slice.
    #[serde(rename = "slice.merge.succeeded", rename_all = "kebab-case")]
    SliceMergeSucceeded {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
    },
    /// `specify slice merge` refused to fold the slice into the
    /// baseline. Fires on the validator
    /// outcome, not on a merge report. `reason` carries a short human
    /// reason or finding code so the journal records why the merge
    /// stalled.
    #[serde(rename = "slice.merge.failed", rename_all = "kebab-case")]
    SliceMergeFailed {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// Short human reason / finding code for the failure.
        reason: String,
    },
    /// `specify source survey --phase finalize` validated and merged
    /// one source's lead set into `discovery.md`. The plan-time peer
    /// of [`Self::SliceExtractCompleted`]; one event per `(source,
    /// survey)` run. CLI-owned.
    #[serde(rename = "source.survey.completed", rename_all = "kebab-case")]
    SourceSurveyCompleted {
        /// Source key from `plan.yaml.sources.<key>`.
        source: String,
        /// Adapter name (kebab-case; mirrors `adapter.yaml.name`).
        adapter: String,
    },
    /// A source adapter ran one operation under agent execution
    /// (`execution: agent`). One event per `(source, operation)`
    /// pair; `operation` is the closed [`SourceOperation`] enum
    /// (`survey | extract`).
    #[serde(rename = "source.execution.agent", rename_all = "kebab-case")]
    SourceExecutionAgent {
        /// Source key from `plan.yaml.sources.<key>`.
        source: String,
        /// Adapter name (kebab-case; mirrors `adapter.yaml.name`).
        adapter: String,
        /// Which operation ran (`survey` at plan time, `extract` at
        /// slice time).
        operation: SourceOperation,
    },
    /// A target adapter ran one operation under agent execution. The
    /// `build` verb emits this per agent invocation.
    /// Unlike [`Self::SourceExecutionAgent`], which
    /// fans out over the `(source, operation)` pair, the build verb
    /// derives `(slice, target)` from the bound project — `build` is
    /// the only agent-dispatched target operation that emits this event
    /// in v1, so the payload stays minimal at `{ slice, target }`.
    #[serde(rename = "target.execution.agent", rename_all = "kebab-case")]
    TargetExecutionAgent {
        /// Slice id under `plan.yaml.slices[].name`.
        slice: SliceName,
        /// Target name (`omnia`, `vectis`, …) the build dispatched to.
        target: String,
    },
    /// runtime capture claim — target's `build` finished replay.
    /// Payload mirrors the `replay:` block written into the
    /// slice's `metadata.yaml`. Optional in v1 (targets that have
    /// not implemented the hook do not emit this event).
    #[serde(rename = "slice.replay.completed", rename_all = "kebab-case")]
    SliceReplayCompleted {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// Replay-runner identity (e.g. `omnia-target@1.4 (cargo nextest)`).
        runner: String,
        /// Number of replay scenarios that passed.
        passed: usize,
        /// Number of replay scenarios that failed.
        failed: usize,
        /// Number of replay scenarios the runner skipped.
        skipped: usize,
    },
    /// per-slice authority override — operator set or cleared a per-slice
    /// `authority-override` map at Gate 1. CLI-driven via
    /// `specify plan create --authority-override`,
    /// `specify plan amend --authority-override`, or the matching
    /// `--clear-*` flags.
    #[serde(rename = "plan.amend.authority-override", rename_all = "kebab-case")]
    PlanAmendAuthorityOverride {
        /// Plan name from `plan.yaml.name`.
        plan_name: PlanName,
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// Closed action discriminator.
        action: AuthorityOverrideAction,
        /// Claim kind the action touched (the closed-enum key under
        /// `slices[].authority-override`).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        claim_kind: Option<String>,
        /// Source key the override now points at, when `action` is
        /// [`AuthorityOverrideAction::Set`]; absent on clear actions.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        source: Option<String>,
    },
    /// `specify plan propose --from` validated the agent reconciliation
    /// response and wrote `plan.yaml.slices[]`. One indivisible event
    /// per successful invocation — the `/spec:plan` skill never calls
    /// `specify journal emit` here.
    #[serde(rename = "plan.reconcile.completed", rename_all = "kebab-case")]
    PlanReconcileCompleted {
        /// Plan name from `plan.yaml.name`.
        plan_name: PlanName,
        /// Count of `plan.yaml.slices[]` rows written.
        slice_count: usize,
        /// Slice names, in the agent's `slices[]` response order.
        slice_names: Vec<SliceName>,
    },
    /// A slice merged into the baseline and its working directory was
    /// archived. This is the durable **outcome-ledger** entry
    /// (DECISIONS.md §"History via git plus an outcome ledger"): the
    /// append-only journal records what merged, when, which baseline
    /// specs it touched, a one-line outcome summary, and the git SHA
    /// the baseline sat at. The archived slice folder under
    /// `.specify/archive/` is a prunable convenience cache
    /// (`specify archive prune`), not the system of record — this
    /// event plus git history of `.specify/specs/` is.
    #[serde(rename = "slice.archive.created", rename_all = "kebab-case")]
    SliceArchiveCreated {
        /// Slice id under `plan.yaml.slices[].name`.
        slice_name: SliceName,
        /// Baseline spec/composition names this slice merged into, in
        /// the merge engine's `(class, name)` order.
        touched_specs: Vec<String>,
        /// One-line human summary of the merge operations (the same
        /// text stamped into the archived slice's `metadata.yaml`
        /// merge outcome).
        outcome_summary: String,
        /// Git HEAD SHA after the merge, when the project is a git
        /// repository; absent otherwise (best-effort, never fatal).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        merge_sha: Option<String>,
        /// `DEC-NNNN` ids promoted into the Decision Record catalogue by
        /// this merge, in slug order. Empty stays off the wire;
        /// this is the durable ledger of promoted decisions alongside git
        /// history of `.specify/decisions/`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        decisions: Vec<String>,
    },
    /// `specify upgrade` self-updated the CLI binary. The new binary
    /// writes the event; `from` is the version observed before the
    /// upgrade, `to` the version now running, `channel` the resolved
    /// install channel (`cargo | brew | binary`).
    #[serde(rename = "cli.upgraded", rename_all = "kebab-case")]
    CliUpgraded {
        /// Version observed before the upgrade.
        from: String,
        /// Version now running.
        to: String,
        /// Resolved install channel (`cargo | brew | binary`).
        channel: String,
    },
    /// `specify plugins refresh` invalidated the Cursor plugin cache.
    /// `deleted_paths` are the cache directories removed (wire:
    /// `deleted-paths`); `marketplace` is the resolved marketplace file
    /// path whose top-level `name` scoped the deletion.
    #[serde(rename = "plugins.refreshed", rename_all = "kebab-case")]
    PluginsRefreshed {
        /// Cache directories removed (wire: `deleted-paths`).
        deleted_paths: Vec<String>,
        /// Resolved marketplace file path whose top-level `name` scoped
        /// the deletion.
        marketplace: String,
    },
    /// `specify workspace sync` materialised the selected workspace
    /// slots and regenerated `topology.lock`. Fires once per
    /// successful sync; the registry-less no-op path emits nothing.
    #[serde(rename = "workspace.sync.completed", rename_all = "kebab-case")]
    WorkspaceSyncCompleted {
        /// Registry project names the sync materialised, in registry
        /// (selection) order.
        projects: Vec<String>,
    },
    /// `specify workspace push` pushed the active plan's
    /// `specify/<plan-name>` branch across the selected projects with
    /// no failed project. Dry runs and failed pushes emit nothing.
    #[serde(rename = "workspace.push.completed", rename_all = "kebab-case")]
    WorkspacePushCompleted {
        /// Plan name from `plan.yaml.name`.
        plan_name: PlanName,
        /// Branch the push targeted (`specify/<plan-name>`).
        branch: String,
        /// Registry project names the push covered, in selection order.
        projects: Vec<String>,
    },
    /// `specify lint` finished a scan. The payload carries the scan
    /// scope, wall-clock duration, per-status counts, and the
    /// CLI exit code the scan resolved to. Emission is
    /// wired in the scanner; this variant exists so the taxonomy is
    /// closed even before the emitter ships.
    ///
    /// Field names on the wire are `snake_case` to match the journal
    /// payload example verbatim (`duration_ms`, `false_positive`,
    /// `exit_code`); this is the one variant in the
    /// taxonomy that does not project through `rename_all =
    /// "kebab-case"`, because that payload shape is the wire contract
    /// consumers will read.
    #[serde(rename = "lint-completed")]
    LintCompleted(LintCompletedPayload),
}

/// Payload for [`EventKind::LintCompleted`]. The journal event
/// contract pins the field set and the `snake_case` wire names.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintCompletedPayload {
    /// Scope of the scan — which target, slice, or artifact the run
    /// was narrowed to. All three sub-fields are optional; a
    /// project-wide scan leaves them `null`.
    pub scope: LintScope,
    /// Wall-clock duration of the scan in milliseconds.
    pub duration_ms: u64,
    /// Per-status counts. The scanner emits `open`, `ignored`, and
    /// `false_positive`.
    pub counts: LintCounts,
    /// CLI exit code the scan resolved to (status-aware severity per
    /// the exit and presentation semantics). `0` on clean
    /// scans, `2` when an `open` finding of `important` or `critical`
    /// severity remains.
    pub exit_code: i32,
}

/// Scan-scope sub-object on [`LintCompletedPayload`]. Each field is
/// optional and serialised as `null` when absent so the wire shape
/// matches the payload example verbatim.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintScope {
    /// Target name (`omnia`, `vectis`, …) when the scan was narrowed
    /// to a single target; `None` for project-wide scans.
    pub target: Option<String>,
    /// Slice id from `plan.yaml.slices[].name` when the scan was
    /// narrowed to one slice (e.g. `specify lint project --slice <name>`).
    pub slice: Option<String>,
    /// Artifact path (relative to project root) when the scan was
    /// narrowed to a single artifact; `None` otherwise.
    pub artifact: Option<String>,
}

/// Per-status finding counts on [`LintCompletedPayload`]. Current
/// emitters fill the three buckets named here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintCounts {
    /// `status: open` count — findings that block CI by default when
    /// they also carry `severity: critical` or `severity: important`.
    pub open: u32,
    /// `status: ignored` count — findings demoted by a matching
    /// `specify-ignore` directive.
    pub ignored: u32,
    /// `status: false-positive` count — findings demoted by a
    /// `specify-ignore` directive whose rationale begins with
    /// `false-positive:`.
    pub false_positive: u32,
}

/// Every wire event id in the closed [`EventKind`] taxonomy, sorted.
///
/// This is the inventory `specify contract dump` publishes and the
/// `cli-contract` lint kind checks citations against. The
/// `wire_event_ids` test in `journal/tests.rs` serialises one sample
/// per variant to prove the table matches the serde renames; adding a
/// variant without updating this table fails that test.
pub const WIRE_EVENT_IDS: &[&str] = &[
    "cli.upgraded",
    "lint-completed",
    "plan.amend.authority-override",
    "plan.amend.divergence",
    "plan.entry.advanced",
    "plan.reconcile.completed",
    "plan.transition.approved",
    "plan.transition.undone",
    "plugins.refreshed",
    "slice.archive.created",
    "slice.build.failed",
    "slice.build.started",
    "slice.build.succeeded",
    "slice.extract.completed",
    "slice.merge.failed",
    "slice.merge.started",
    "slice.merge.succeeded",
    "slice.replay.completed",
    "slice.synthesis.conflict",
    "slice.synthesis.divergence",
    "slice.synthesis.unknown",
    "slice.synthesize.agent",
    "slice.synthesize.completed",
    "slice.synthesize.failed",
    "slice.synthesize.started",
    "slice.transition.refined",
    "source.execution.agent",
    "source.survey.completed",
    "target.execution.agent",
    "workspace.push.completed",
    "workspace.sync.completed",
];

/// Closed `actor` enum on [`EventKind::PlanTransitionApproved`] —
/// who drove the Gate-1 stamp.
///
/// Self-reported through `specify plan transition --actor` (default
/// `operator`), so the value is grading evidence for eval probes
/// (`gate-1-not-auto-stamped`), not an enforcement surface. Defaults
/// to [`Actor::Operator`] both at the flag and at deserialisation so
/// journal lines written before the field existed keep parsing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum Actor {
    /// A human operator ran the verb (the default).
    #[default]
    Operator,
    /// An agent ran the verb on the operator's behalf.
    Agent,
}

impl std::str::FromStr for Actor {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "operator" => Ok(Self::Operator),
            "agent" => Ok(Self::Agent),
            other => Err(format!("unknown actor `{other}`; expected `operator` or `agent`")),
        }
    }
}

/// Closed `action` enum on [`EventKind::PlanAmendAuthorityOverride`].
///
/// Mirrors the per-kind mutations emitted by the CLI surface
/// (`--authority-override`, `--clear-authority-override`, and the
/// per-kind expansion of `--clear-authority-overrides`).
///
/// Variants are declared in the documented sort order `Set < Clear`
/// so batched `mutate_authority_overrides` callers emit set-then-clear
/// journal events; the `set_sorts_before_clear` test guards drift.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, strum::Display,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum AuthorityOverrideAction {
    /// `--authority-override <slice> <kind>=<key>` set the value.
    Set,
    /// `--clear-authority-override <slice> <kind>` removed one entry.
    Clear,
}

#[cfg(test)]
mod authority_override_action_tests {
    use super::AuthorityOverrideAction;

    #[test]
    fn set_sorts_before_clear() {
        let mut actions = [AuthorityOverrideAction::Clear, AuthorityOverrideAction::Set];
        actions.sort();
        assert_eq!(
            actions,
            [AuthorityOverrideAction::Set, AuthorityOverrideAction::Clear],
            "Set MUST sort before Clear so batched plan-amend journal writes \
             replay the operator's intent set-then-clear; the wire contract \
             depends on this ordering (see PlanAmendAuthorityOverride)."
        );
    }
}

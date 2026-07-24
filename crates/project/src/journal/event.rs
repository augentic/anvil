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
use crate::name::{PlanName, SliceName};
use crate::plan::Divergence;

/// One row of the journal. Serialises as `{ timestamp, event,
/// payload }` — workflow §Wire format pins `timestamp` first so a
/// `head -1` on the file is enough to confirm the run window.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Second-precision UTC timestamp (`%Y-%m-%dT%H:%M:%SZ`).
    #[serde(with = "crate::serde_time::rfc3339")]
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
    /// Gate 1 cleared — `specify plan approve`.
    #[serde(rename = "plan.transition.approved", rename_all = "kebab-case")]
    PlanTransitionApproved {
        /// Governing plan.
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
        /// Governing plan.
        plan_name: PlanName,
        /// Affected entry.
        slice_name: SliceName,
        /// Status the entry held before the undo.
        from: crate::plan::Status,
        /// Status the entry holds after the undo.
        to: crate::plan::Status,
    },
    /// `specify plan next` advanced one entry `pending → in-progress`
    /// (the sole writer of per-entry `in-progress`). Fires only when
    /// an entry actually advanced — returning the already-active entry
    /// or reporting drained/stuck emits nothing, so the *absence* of
    /// this event over a window is probeable evidence that the execute
    /// loop parked rather than advancing.
    #[serde(rename = "plan.entry.advanced", rename_all = "kebab-case")]
    PlanEntryAdvanced {
        /// Governing plan.
        plan_name: PlanName,
        /// Advanced entry.
        slice_name: SliceName,
    },
    /// Stamped `slices[].divergence` via
    /// `specify plan amend --divergence <likely|accepted|rejected>`.
    /// The CLI is the single writer. In the reconcile flow the
    /// `/spec:plan` agent stages `likely`
    /// through this event after the reconcile write; the operator later
    /// flips `accepted` / `rejected` the same way. This is the only
    /// path that writes the `divergence` field.
    #[serde(rename = "plan.amend.divergence", rename_all = "kebab-case")]
    PlanAmendDivergence {
        /// Governing plan.
        plan_name: PlanName,
        /// Affected slice.
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
        /// Affected slice.
        slice_name: SliceName,
    },
    /// The `source extract` finalize tail validated and
    /// persisted one source-bound Evidence document. One event per
    /// `(source, slice)` pair. CLI-owned — the `/spec:refine` skill
    /// never emits this via `specify journal emit`.
    #[serde(rename = "slice.extract.completed", rename_all = "kebab-case")]
    SliceExtractCompleted {
        /// Affected slice.
        slice_name: SliceName,
        /// Extracted source key.
        source: String,
    },
    /// `[conflict]` on a requirement in `spec.md` — same-authority
    /// disagreement the operator must reconcile. Emitted by
    /// `specify slice validate` after a successful run.
    #[serde(rename = "slice.synthesis.conflict", rename_all = "kebab-case")]
    SliceSynthesisConflict {
        /// Affected slice.
        slice_name: SliceName,
        /// `ID:` value on the tagged requirement block.
        requirement_id: String,
    },
    /// `[divergence]` on a requirement in `spec.md` — cross-authority
    /// disagreement preserved as inline commentary. Emitted by
    /// `specify slice validate` after a successful run.
    #[serde(rename = "slice.synthesis.divergence", rename_all = "kebab-case")]
    SliceSynthesisDivergence {
        /// Affected slice.
        slice_name: SliceName,
        /// `ID:` value on the tagged requirement block.
        requirement_id: String,
    },
    /// `[unknown]` on a requirement in `spec.md` — a gap the operator
    /// must close before the requirement is meaningful. Emitted by
    /// `specify slice validate` after a successful run.
    #[serde(rename = "slice.synthesis.unknown", rename_all = "kebab-case")]
    SliceSynthesisUnknown {
        /// Affected slice.
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
        /// Affected slice.
        slice_name: SliceName,
    },
    /// Synthesis dispatched to the agent. Synthesis is always
    /// agent-driven; this signal fires on the dry-run inputs phase so
    /// the journal records the handoff.
    #[serde(rename = "slice.synthesize.agent", rename_all = "kebab-case")]
    SliceSynthesizeAgent {
        /// Affected slice.
        slice_name: SliceName,
    },
    /// Slice synthesis finished and the artifacts were persisted.
    /// `artifacts` lists the persisted
    /// relative paths (`proposal.md`, `specs/<domain>/spec.md`,
    /// `design.md`, `tasks.md`, `model.yaml`).
    #[serde(rename = "slice.synthesize.completed", rename_all = "kebab-case")]
    SliceSynthesizeCompleted {
        /// Affected slice.
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
        /// Affected slice.
        slice_name: SliceName,
        /// Short human reason / finding code for the failure.
        reason: String,
    },
    /// `/spec:build` started implementing the slice — the target
    /// adapter's `build` brief began running against the refined
    /// artifacts. One event per slice.
    #[serde(rename = "slice.build.started", rename_all = "kebab-case")]
    SliceBuildStarted {
        /// Affected slice.
        slice_name: SliceName,
    },
    /// `/spec:build` finished implementing the slice — the target
    /// adapter's `build` brief completed and the slice is ready for
    /// `/spec:merge`. One event per slice.
    #[serde(rename = "slice.build.succeeded", rename_all = "kebab-case")]
    SliceBuildSucceeded {
        /// Affected slice.
        slice_name: SliceName,
    },
    /// `/spec:build` stopped before the slice was implemented.
    /// `reason` carries a short human
    /// reason or finding code so the journal records why the build
    /// stalled.
    #[serde(rename = "slice.build.failed", rename_all = "kebab-case")]
    SliceBuildFailed {
        /// Affected slice.
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
        /// Affected slice.
        slice_name: SliceName,
    },
    /// `specify slice merge` validated and applied the slice's deltas
    /// to the baseline. Fires on the
    /// validator outcome, not on a merge report. One event per slice.
    #[serde(rename = "slice.merge.succeeded", rename_all = "kebab-case")]
    SliceMergeSucceeded {
        /// Affected slice.
        slice_name: SliceName,
    },
    /// `specify slice merge` refused to fold the slice into the
    /// baseline. Fires on the validator
    /// outcome, not on a merge report. `reason` carries a short human
    /// reason or finding code so the journal records why the merge
    /// stalled.
    #[serde(rename = "slice.merge.failed", rename_all = "kebab-case")]
    SliceMergeFailed {
        /// Affected slice.
        slice_name: SliceName,
        /// Short human reason / finding code for the failure.
        reason: String,
    },
    /// The guest merge orchestrator skipped the workspace-clone git
    /// commit leg — the engine guest owns no git surface, so the
    /// merge lands on `.specify/` state only. Explicit
    /// so a journal reader can tell a guest merge (no `merge-sha` on
    /// its `slice.archive.created`) from a native merge that simply ran
    /// outside a clone. Native `specify slice merge` never emits this.
    #[serde(rename = "slice.merge.commit-skipped", rename_all = "kebab-case")]
    SliceMergeCommitSkipped {
        /// Affected slice.
        slice_name: SliceName,
    },
    /// The target's postflight merge gate raised a blocking finding
    /// **after** the deterministic commit: the slice is already merged,
    /// archived, and stamped `done`, so this is a terminal diagnostic —
    /// never a rollback. `reason` carries a short human reason or
    /// finding code; the merged baseline stands.
    #[serde(rename = "slice.merge.postflight-failed", rename_all = "kebab-case")]
    SliceMergePostflightFailed {
        /// Affected slice — already merged and archived.
        slice_name: SliceName,
        /// Short human reason / finding code for the failed gate.
        reason: String,
    },
    /// The `source survey` finalize tail validated and merged
    /// one source's lead set into `discovery.md`. The plan-time peer
    /// of [`Self::SliceExtractCompleted`]; one event per `(source,
    /// survey)` run. CLI-owned.
    #[serde(rename = "source.survey.completed", rename_all = "kebab-case")]
    SourceSurveyCompleted {
        /// Surveyed source key.
        source: String,
        /// Adapter name (kebab-case; the resolved adapter identity).
        adapter: String,
    },
    /// A source adapter ran one operation under agent execution
    /// (`execution: agent`). One event per `(source, operation)`
    /// pair; `operation` is the closed [`SourceOperation`] enum
    /// (`survey | extract`).
    #[serde(rename = "source.execution.agent", rename_all = "kebab-case")]
    SourceExecutionAgent {
        /// Dispatched source key.
        source: String,
        /// Adapter name (kebab-case; the resolved adapter identity).
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
        /// Affected slice.
        slice: SliceName,
        /// Target name (`omnia`, `vectis`, …) the build dispatched to.
        target: String,
    },
    /// per-slice authority override — operator set or cleared a per-slice
    /// `authority-override` map at Gate 1. CLI-driven via
    /// `specify plan add --authority-override`,
    /// `specify plan amend --authority-override`, or the matching
    /// `--clear-*` flags.
    #[serde(rename = "plan.amend.authority-override", rename_all = "kebab-case")]
    PlanAmendAuthorityOverride {
        /// Governing plan.
        plan_name: PlanName,
        /// Affected slice.
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
    /// The `plan author` reconcile kernel validated the agent
    /// reconciliation response and wrote `plan.yaml.slices[]`. One indivisible event
    /// per successful invocation — the `/spec:plan` skill never calls
    /// `specify journal emit` here.
    #[serde(rename = "plan.reconcile.completed", rename_all = "kebab-case")]
    PlanReconcileCompleted {
        /// Governing plan.
        plan_name: PlanName,
        /// Count of `plan.yaml.slices[]` rows written.
        slice_count: usize,
        /// Slice names, in the agent's `slices[]` response order.
        slice_names: Vec<SliceName>,
    },
    /// A slice merged into the baseline and its working directory was
    /// archived. This is the durable **outcome-ledger** entry: the
    /// append-only journal records what merged, when, which baseline
    /// specs it touched, a one-line outcome summary, and the git SHA
    /// the baseline sat at. The archived slice folder under
    /// `.specify/archive/` is a prunable convenience cache
    /// (`specify archive prune`), not the system of record — this
    /// event plus git history of `.specify/specs/` is.
    #[serde(rename = "slice.archive.created", rename_all = "kebab-case")]
    SliceArchiveCreated {
        /// Archived slice.
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
}

/// Closed `actor` enum on [`EventKind::PlanTransitionApproved`] —
/// who drove the Gate-1 stamp.
///
/// Self-reported through `specify plan approve --actor` (default
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
/// so batched `authority_override::mutate` callers emit set-then-clear
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

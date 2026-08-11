//! Closed journal event taxonomy and wire DTOs.
//!
//! Wire format is locked: dotted kebab-case event ids, kebab-case
//! payload fields; Rust variants reach the wire via `#[serde(rename)]`.

use artifacts::spec::provenance::RequirementStatus;
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::adapter::operation::SourceOperation;
use crate::name::{PlanName, SliceName};
use crate::plan::Divergence;

/// One row of a per-writer event log.
///
/// Serialises as `{ timestamp, writer, sequence, event, payload }` —
/// RFC-86 pins `timestamp` first so a `head -1` on the file is enough
/// to confirm the run window; `writer` + `sequence` identify the line
/// inside that writer's append-only file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    /// Second-precision UTC timestamp (`%Y-%m-%dT%H:%M:%SZ`).
    #[serde(with = "crate::serde_time::rfc3339")]
    pub timestamp: Timestamp,
    /// Journal writer that appended this line (`EMERY_WRITER` or the stable
    /// local default). Empty only on in-memory values before
    /// [`super::append_for`] stamps the wire fields.
    ///
    /// Deserialise accepts the prior `actor` wire key so existing
    /// `.emery/events/*.jsonl` lines remain in the union after the rename.
    #[serde(alias = "actor")]
    pub writer: String,
    /// Monotonic per-writer sequence (1-based) inside that writer's
    /// `.jsonl` file. Zero only on in-memory values before append
    /// stamps the wire fields.
    pub sequence: u64,
    /// Event id + payload, adjacently tagged so `event` and `payload`
    /// sit side by side in the JSON object.
    #[serde(flatten)]
    pub kind: EventKind,
}

impl Event {
    /// Build an [`Event`] at `timestamp` carrying `kind`.
    ///
    /// `writer` and `sequence` stay unset (`""` / `0`) until
    /// [`super::append_for`] (or [`super::append_one`]) stamps them
    /// for the calling writer's file. Tests pin the timestamp;
    /// production callers pass an injected `now`.
    #[must_use]
    pub const fn new(timestamp: Timestamp, kind: EventKind) -> Self {
        Self {
            timestamp,
            writer: String::new(),
            sequence: 0,
            kind,
        }
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
    /// The execute loop's advance step moved one entry
    /// `pending → in-progress` (the sole writer of per-entry
    /// `in-progress`). Fires only when an entry actually advanced —
    /// returning the already-active entry or reporting drained/stuck
    /// emits nothing, so the *absence* of this event over a window is
    /// probeable evidence that the execute loop parked rather than
    /// advancing.
    #[serde(rename = "plan.entry.advanced", rename_all = "kebab-case")]
    PlanEntryAdvanced {
        /// Governing plan.
        plan_name: PlanName,
        /// Advanced entry.
        slice_name: SliceName,
    },
    /// Stamped `slices[].divergence` via
    /// `emery plan amend --divergence <likely|accepted|rejected>`.
    /// The CLI is the single writer. In the reconcile flow the
    /// `/emery:plan` agent stages `likely`
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
    /// slice is ready for the build phase.
    #[serde(rename = "slice.transition.refined", rename_all = "kebab-case")]
    SliceTransitionRefined {
        /// Affected slice.
        slice_name: SliceName,
    },
    /// The `source extract` finalize tail validated and
    /// persisted one source-bound Evidence document. One event per
    /// `(source, slice)` pair. CLI-owned — appended by the extract
    /// orchestration itself, never by a skill.
    #[serde(rename = "slice.extract.completed", rename_all = "kebab-case")]
    SliceExtractCompleted {
        /// Affected slice.
        slice_name: SliceName,
        /// Extracted source key.
        source: String,
    },
    /// `[conflict]` on a requirement in `spec.md` — same-authority
    /// disagreement the operator must reconcile. Emitted by
    /// `emery slice validate` after a successful run.
    #[serde(rename = "slice.synthesis.conflict", rename_all = "kebab-case")]
    SliceSynthesisConflict {
        /// Affected slice.
        slice_name: SliceName,
        /// `ID:` value on the tagged requirement block.
        requirement_id: String,
    },
    /// `[divergence]` on a requirement in `spec.md` — cross-authority
    /// disagreement preserved as inline commentary. Emitted by
    /// `emery slice validate` after a successful run.
    #[serde(rename = "slice.synthesis.divergence", rename_all = "kebab-case")]
    SliceSynthesisDivergence {
        /// Affected slice.
        slice_name: SliceName,
        /// `ID:` value on the tagged requirement block.
        requirement_id: String,
    },
    /// `[unknown]` on a requirement in `spec.md` — a gap the operator
    /// must close before the requirement is meaningful. Emitted by
    /// `emery slice validate` after a successful run.
    #[serde(rename = "slice.synthesis.unknown", rename_all = "kebab-case")]
    SliceSynthesisUnknown {
        /// Affected slice.
        slice_name: SliceName,
        /// `ID:` value on the tagged requirement block.
        requirement_id: String,
    },
    /// Slice synthesis began — the refine phase started folding the
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
    /// The build phase started implementing the slice — the target
    /// adapter's `build` brief began running against the refined
    /// artifacts. One event per slice.
    #[serde(rename = "slice.build.started", rename_all = "kebab-case")]
    SliceBuildStarted {
        /// Affected slice.
        slice_name: SliceName,
    },
    /// The build phase finished implementing the slice — the target
    /// adapter's `build` brief completed and the slice is ready for
    /// the merge phase. One event per slice.
    #[serde(rename = "slice.build.succeeded", rename_all = "kebab-case")]
    SliceBuildSucceeded {
        /// Affected slice.
        slice_name: SliceName,
    },
    /// The build phase stopped before the slice was implemented.
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
    /// One engine-selected build phase returned a report that was
    /// persisted under the attempt tree (RFC-90 D6). Ordinal evidence
    /// inside the `slice.build.started` / `.succeeded` / `.failed`
    /// envelope — never lifecycle authority. `elapsed-ms` is
    /// engine-measured raw telemetry outside the report digest.
    #[serde(rename = "slice.build.phase-completed", rename_all = "kebab-case")]
    SliceBuildPhaseCompleted {
        /// Affected slice.
        slice_name: SliceName,
        /// Attempt ordinal (the numeric id of
        /// `build/attempts/<attempt>/`, 1-based).
        attempt: u32,
        /// Phase ordinal within the attempt (the numeric prefix of
        /// `phases/<ordinal>-<operation>.yaml`, 1-based).
        ordinal: u32,
        /// Kebab-case phase operation (`build | verify | repair |
        /// review`).
        operation: String,
        /// Kebab-case report-level phase source (`deterministic |
        /// model-assisted | hybrid`).
        source: String,
        /// `sha256:<hex>` digest of the persisted phase-report bytes.
        report_digest: String,
        /// Engine-measured wall-clock duration of the dispatch, in
        /// milliseconds. Outside the report digest.
        elapsed_ms: u64,
    },
    /// The merge phase began folding the slice's deltas into the
    /// baseline. The `slice.merge.*` pair
    /// fires on the merge validator outcome, not on a
    /// merge report. One event per slice.
    #[serde(rename = "slice.merge.started", rename_all = "kebab-case")]
    SliceMergeStarted {
        /// Affected slice.
        slice_name: SliceName,
    },
    /// The merge phase validated and applied the slice's deltas
    /// to the baseline. Fires on the
    /// validator outcome, not on a merge report. One event per slice.
    #[serde(rename = "slice.merge.succeeded", rename_all = "kebab-case")]
    SliceMergeSucceeded {
        /// Affected slice.
        slice_name: SliceName,
    },
    /// The merge phase refused to fold the slice into the
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
    /// merge lands on `.emery/` state only. Explicit
    /// so a journal reader can tell a guest merge (no `merge-sha` on
    /// its `slice.archive.created`) from a native merge that simply ran
    /// outside a clone. A native merge phase never emits this.
    #[serde(rename = "slice.merge.commit-skipped", rename_all = "kebab-case")]
    SliceMergeCommitSkipped {
        /// Affected slice.
        slice_name: SliceName,
    },
    /// Interim code delivery (RFC-87, pre-RFC-89): after the target's
    /// postflight gate passed, the merge orchestration materialized
    /// the slice's accepted result snapshot onto the product tree.
    /// Deleted when publication sets (RFC-89) own the final seal.
    #[serde(rename = "slice.code.applied", rename_all = "kebab-case")]
    SliceCodeApplied {
        /// Affected slice.
        slice_name: SliceName,
        /// The applied result snapshot (`sha256:<hex>`).
        snapshot: String,
    },
    /// The target's postflight merge gate raised a blocking finding
    /// **after** `target.merge.wave-committed` (RFC-86 D9): the member
    /// is already merged, so this is a terminal diagnostic — never a
    /// rollback. `reason` carries a short human reason or finding code;
    /// the merged baseline stands.
    #[serde(rename = "target.merge.wave-postflight-failed", rename_all = "kebab-case")]
    TargetMergeWavePostflightFailed {
        /// Target key under `.emery/targets/`.
        target: String,
        /// Wave manifest content digest (`sha256:<64 hex>`).
        digest: String,
        /// Sole member's slice — already merged and archived.
        slice_name: SliceName,
        /// Short human reason / finding code for the failed gate.
        reason: String,
    },
    /// Deterministic wave commit finalized requirement identity maps
    /// (RFC-86 D5 / D9). Projects the member merged; failures before
    /// this fact leave no merged projection. Carries every local→
    /// baseline `REQ-NNN` mapping for the wave's sole member.
    #[serde(rename = "target.merge.wave-committed", rename_all = "kebab-case")]
    TargetMergeWaveCommitted {
        /// Target key under `.emery/targets/`.
        target: String,
        /// Wave manifest content digest (`sha256:<64 hex>`).
        digest: String,
        /// Sole member's slice name.
        slice_name: SliceName,
        /// Closed-plan commit-authorization epoch (may differ from the
        /// wave's build-authorization; serial execution normally reuses
        /// the same epoch).
        commit_authorization: FactEpochRef,
        /// Slice-local id → final baseline `REQ-NNN` for every
        /// requirement in the member.
        identity_maps: Vec<IdentityMap>,
        /// Deferred member set the wave carried into the baseline
        /// (RFC-86a D5) — the committed audit trail names exactly
        /// which debt this wave accepted. Empty when nothing was
        /// deferred.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        deferred: Vec<DeferredMember>,
    },
    /// Target postflight gate succeeded after wave commit (RFC-86 D9).
    #[serde(rename = "target.merge.wave-succeeded", rename_all = "kebab-case")]
    TargetMergeWaveSucceeded {
        /// Target key under `.emery/targets/`.
        target: String,
        /// Wave manifest content digest (`sha256:<64 hex>`).
        digest: String,
        /// Sole member's slice name.
        slice_name: SliceName,
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
    /// `authority-override` map during plan review. CLI-driven via
    /// `emery plan add --authority-override`,
    /// `emery plan amend --authority-override`, or the matching
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
    /// per successful invocation — appended by the reconcile kernel
    /// itself, never by a skill.
    #[serde(rename = "plan.reconcile.completed", rename_all = "kebab-case")]
    PlanReconcileCompleted {
        /// Governing plan.
        plan_name: PlanName,
        /// Count of `plan.yaml.slices[]` rows written.
        slice_count: usize,
        /// Slice names, in the agent's `slices[]` response order.
        slice_names: Vec<SliceName>,
    },
    /// `emery plan execute` acknowledged a sticky
    /// `merge-postflight-failed` stop and is continuing the queue.
    /// Clears the plan-wide postflight debt projected by `plan status`
    /// until the next `target.merge.wave-postflight-failed`. No new CLI
    /// verb — re-running execute is the ack.
    #[serde(rename = "plan.merge-postflight.acknowledged", rename_all = "kebab-case")]
    PlanMergePostflightAcknowledged {
        /// Slice whose postflight debt was acknowledged — already
        /// merged, archived, and stamped `done`.
        slice_name: SliceName,
    },
    /// A slice merged into the baseline and its working directory was
    /// archived. This is the durable **outcome-ledger** entry: the
    /// append-only journal records what merged, when, which baseline
    /// specs it touched, a one-line outcome summary, and the git SHA
    /// the baseline sat at. The archived slice folder under
    /// `.emery/archive/` is a prunable convenience cache
    /// (`emery archive prune`), not the system of record — this
    /// event plus git history of `.emery/specs/` is.
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
        /// history of `.emery/decisions/`.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        decisions: Vec<String>,
    },
    /// A journal writer claimed exclusive ownership of one slice
    /// (RFC-86 D7 / D23). The claimant is the event's envelope
    /// `writer`, not a payload field. Claims never create authorization.
    #[serde(rename = "slice.claimed", rename_all = "kebab-case")]
    SliceClaimed {
        /// Claimed slice.
        slice_name: SliceName,
    },
    /// The claiming writer released its exclusive ownership of one
    /// slice. Only a live claim by the releasing envelope `writer`
    /// clears ownership under the claim kernel.
    #[serde(rename = "slice.released", rename_all = "kebab-case")]
    SliceReleased {
        /// Released slice.
        slice_name: SliceName,
    },
    /// An immutable one-member target wave was written before build
    /// (RFC-86 D9). The manifest lives at
    /// `.emery/targets/<target>/waves/<digest>.yaml`; `digest` is the
    /// content address (`sha256:…`) of that YAML.
    #[serde(rename = "target.wave.opened", rename_all = "kebab-case")]
    TargetWaveOpened {
        /// Target key under `.emery/targets/` (project name in the
        /// in-place cut).
        target: String,
        /// Manifest content digest (`sha256:<64 hex>`).
        digest: String,
        /// The sole member's slice name.
        slice_name: SliceName,
    },
    /// `emery plan execute` opened an authorization epoch at start
    /// (RFC-86 D6 / D22). Presence projects the Authorized milestone.
    /// Never named `plan.approved`.
    #[serde(rename = "plan.execute.started", rename_all = "kebab-case")]
    PlanExecuteStarted {
        /// Typed `closed-plan` coverage over the reviewed plan.
        coverage: ClosedPlanCoverage,
        /// Detached discovery digest when present (RFC-88).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        discovery_digest: Option<String>,
    },
    /// A typed gap requirement was durably deferred (RFC-86a D2). The
    /// `(slice, requirement-digest)` pair is the disposition match
    /// key; liveness is recomputed against the live model at
    /// projection time, never stored.
    #[serde(rename = "gap.deferred", rename_all = "kebab-case")]
    GapDeferred {
        /// Slice that owns the requirement — scopes the digest join.
        slice: SliceName,
        /// Advisory `REQ-NNN` id at deferral time (presentation only —
        /// a re-refine may renumber ids while the digest holds).
        req: String,
        /// Canonical requirement-body digest (`sha256:<hex>`).
        requirement_digest: String,
        /// The synthesized gate-time reason.
        reason: String,
    },
}

/// Typed `closed-plan` coverage on [`EventKind::PlanExecuteStarted`]
/// (RFC-86 D6). Wire fields use explicit kebab-case renames — container
/// `rename_all` does not reach internally-tagged variant fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum ClosedPlanCoverage {
    /// One reviewed plan digest with per-leaf spec coverage.
    ClosedPlan {
        /// Content digest of the reviewed `plan.yaml`.
        #[serde(rename = "plan-digest")]
        plan_digest: String,
        /// Sorted per-leaf coverage: `existing { digest }` or
        /// `refine-under-epoch`.
        specs: std::collections::BTreeMap<String, LeafSpecCoverage>,
    },
}

/// Per-leaf spec coverage inside [`ClosedPlanCoverage::ClosedPlan`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum LeafSpecCoverage {
    /// Leaf already has a reviewed spec at this digest.
    Existing {
        /// Spec-tree digest (`sha256:…`).
        digest: String,
    },
    /// Authorize refine-before-build for this leaf under the epoch.
    RefineUnderEpoch,
}

/// One deferred requirement in the member set snapshotted on
/// [`EventKind::TargetMergeWaveCommitted`] (RFC-86a D5): the debt the
/// committed wave carried into the baseline.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DeferredMember {
    /// Final baseline `REQ-NNN` assigned at wave commit.
    pub req: String,
    /// Typed gap status of the folded row (`unknown` | `conflict`).
    pub status: RequirementStatus,
    /// Canonical requirement-body digest (`sha256:<hex>`) — the
    /// deferral match key back into the `gap.deferred` facts.
    pub requirement_digest: String,
}

/// One slice-local → baseline requirement identity mapping on
/// [`EventKind::TargetMergeWaveCommitted`] (RFC-86 D5 / D9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct IdentityMap {
    /// Slice-local `REQ-NNN` minted at synthesize.
    pub local: String,
    /// Final baseline `REQ-NNN` assigned at wave commit.
    pub baseline: String,
}

/// Fact-log identity of an authorization epoch (`writer` + 1-based
/// `sequence`), carried on wave commit facts. Same shape as the wave
/// manifest's `build-authorization` / commit-authorization refs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct FactEpochRef {
    /// Writer file that holds the epoch fact.
    ///
    /// Deserialise accepts the prior `actor` wire key (wave
    /// `build-authorization` / commit-authorization refs).
    #[serde(alias = "actor")]
    pub writer: String,
    /// 1-based sequence of the epoch fact in that writer's file.
    pub sequence: u64,
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

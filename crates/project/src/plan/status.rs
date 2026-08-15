//! Read-only `emery plan status` projection.
//!
//! [`plan_status_body`] projects topology, slice artifacts, and the fact
//! union into a deterministic `next-action`; writes nothing.

use serde::Serialize;

use super::gaps::{DebtCounts, GapsBody};

mod project;

pub use project::plan_status_body;

/// Closed next-action verb set on [`StatusBody::action`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum NextActionKind {
    /// The refinement stage (`emery plan refine`, RFC-91) awaits
    /// [`StatusBody::slice`].
    Refine,
    /// The execute loop's build phase awaits [`StatusBody::slice`].
    Build,
    /// The execute loop's merge phase awaits [`StatusBody::slice`].
    Merge,
    /// Every entry is done but a publication member awaits its
    /// worktree (RFC-95 D11); [`StatusBody::target`] names it. The
    /// execute loop reconciles it without opening a new epoch.
    Materialize,
    /// Halt the loop; [`StatusBody::stop`] carries the reason.
    Stop,
    /// No pending or in-progress entries remain and every publication
    /// member is materialized — the only clean exit.
    Drained,
}

/// Closed slice-loop step set for the re-entry fields
/// ([`StatusBody::current_step`] / [`StatusBody::last_completed`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum LoopStep {
    /// The refine phase.
    Refine,
    /// The build phase.
    Build,
    /// The merge phase, including the per-entry `done` stamp.
    Merge,
}

/// Closed stop-reason set on [`StopBody::reason`].
///
/// The loop stops (`refine-failed` / `build-failed` /
/// `merge-conflict` / `merge-postflight-failed`) carry the
/// stop-conditions reference's structured strings; the rest are
/// pre-loop or repair conditions the driver renders the same way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum StopReason {
    /// The awaited refine phase last ended in `slice.synthesize.failed`.
    RefineFailed,
    /// Execute reached an entry without a fresh refinement manifest —
    /// execute never refines (RFC-91); refinement is `emery plan
    /// refine`.
    RefinementRequired,
    /// The awaited build phase last ended in `slice.build.failed`.
    BuildFailed,
    /// The awaited merge phase last ended in `slice.merge.failed`.
    MergeConflict,
    /// The target's postflight merge gate failed after commit — the
    /// entry is already `done` and archived (non-rollback). Sticky
    /// until `emery plan execute` acknowledges.
    MergePostflightFailed,
    /// The active entry's slice was dropped without merging.
    SliceDropped,
    /// The slice merged but the entry is still `in-progress` — the
    /// `done` stamp is missing.
    MergeIncomplete,
    /// Pending entries remain but every one waits on unmet dependencies.
    Stuck,
    /// Refinement Evidence produced an inert boundary proposal. The
    /// leaf is parked until the operator applies it.
    BoundaryEscalation,
    /// Focused resurvey or nearest-domain re-decomposition exhausted
    /// its compiled budget. The leaf is parked.
    RefineBudgetExhausted,
    /// The publication worktree carries uncommitted operator edits
    /// (RFC-95 D11) — materialize refuses to overwrite them.
    PublicationWorktreeDirty,
    /// Provisioning the publication worktree failed on one of the
    /// closed D11 rows (`branch-diverged | branch-checked-out-elsewhere
    /// | destination-conflict | parent-unavailable | clone-failed`).
    #[serde(rename = "publication-provision-failed")]
    #[strum(serialize = "publication-provision-failed")]
    PublicationProvision,
}

impl StopReason {
    /// Operator hint rendered under the stop block — one line, aligned
    /// with the stop-conditions reference's re-entry contract.
    #[must_use]
    pub const fn hint(self) -> &'static str {
        match self {
            Self::RefineFailed => {
                "Fix the failure, then re-run emery plan refine — the drain resumes the \
                 missing or stale refinement. The plan entry stays in-progress."
            }
            Self::RefinementRequired => {
                "Run emery plan refine to generate fresh refinement manifests, then re-run \
                 emery plan execute — execute never refines."
            }
            Self::BuildFailed => {
                "Fix the failure, then re-run emery plan execute — the loop resumes at the \
                 build phase. The plan entry stays in-progress."
            }
            Self::MergeConflict => {
                "Resolve the baseline conflict (or drop the slice with emery plan drop), then \
                 re-run emery plan execute. The plan entry stays in-progress until the merge \
                 lands."
            }
            Self::MergePostflightFailed => {
                "The merge already committed and archived; the plan entry is done. Inspect the \
                 archive merge/postflight.yaml, repair the unclean baseline (hand-fix or a \
                 follow-up slice via /emery:plan), then re-run emery plan execute to acknowledge \
                 and continue."
            }
            Self::SliceDropped => {
                "The slice was dropped; amend or remove the plan entry to unblock the queue."
            }
            Self::MergeIncomplete => {
                "The slice is merged but the entry is still in-progress; re-run emery plan \
                 execute — the merge re-entry stamps the missing done."
            }
            Self::Stuck => {
                "Remaining entries wait on unmet dependencies; complete or amend the blocking \
                 entries."
            }
            Self::BoundaryEscalation => {
                "Refinement wrote an inert boundary proposal under planning/proposals/. Apply it \
                 with emery plan amend --proposal <digest> after quiescing affected work, then \
                 re-run emery plan refine on the new child slices. Re-running refine on this \
                 leaf does not re-synthesize."
            }
            Self::RefineBudgetExhausted => {
                "Focused resurvey or nearest-domain re-decomposition exhausted its budget. \
                 Adjust sources or the bound profile, then re-run emery plan refine."
            }
            Self::PublicationWorktreeDirty => {
                "The publication worktree has uncommitted operator edits. Commit or stash them \
                 (or discard them), then re-run emery plan execute to materialize."
            }
            Self::PublicationProvision => {
                "Provisioning the publication worktree failed; the detail names the member and \
                 the closed reason. Fix the worktree or branch state, then re-run emery plan \
                 execute."
            }
        }
    }
}

/// Stop sub-body on [`StatusBody::stop`], populated when
/// [`StatusBody::action`] is [`NextActionKind::Stop`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct StopBody {
    /// Why the loop must halt.
    pub reason: StopReason,
    /// Failure detail from the journal event payload, when one exists.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// One-line operator hint for this stop.
    pub hint: &'static str,
}

/// Per-status entry counts on [`StatusBody::counts`].
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct StatusCounts {
    /// Entries at `pending`.
    pub pending: usize,
    /// Entries at `in-progress`.
    pub in_progress: usize,
    /// Entries at `done`.
    pub done: usize,
}

/// Wire body for `emery plan status` (text + JSON).
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct StatusBody {
    /// Plan name from `plan.yaml.name`.
    pub plan: String,
    /// Per-status entry counts.
    pub counts: StatusCounts,
    /// Name of the active `in-progress` entry, when one exists.
    pub active: Option<String>,
    /// Rendered projection — `refine|build|merge <slice>`,
    /// `stop <reason>`, or `drained`.
    pub next_action: String,
    /// Closed verb behind [`Self::next_action`].
    pub action: NextActionKind,
    /// Slice the action targets; `None` on `stop`-without-slice
    /// and `drained`.
    pub slice: Option<String>,
    /// Bound target of the targeted entry.
    pub target: Option<String>,
    /// Step the targeted slice is currently at — the awaited
    /// phase, including a phase the loop is stopped on. `None` when no
    /// slice is targeted (`stuck`, `slice-dropped`, `drained`).
    pub current_step: Option<LoopStep>,
    /// Most recent step the targeted slice completed, from artifacts
    /// and success facts (`refined` → `refine`, `built` → `build`, a
    /// landed merge → `merge`). `None` before the first phase
    /// completes or when no slice is targeted.
    pub last_completed: Option<LoopStep>,
    /// Next valid resume point as a literal command — `emery plan
    /// execute` for dispatches and retryable stops (the loop owns
    /// every phase), `/emery:execute` after author (D26),
    /// `/emery:finalize` on drained. `None` when no single command
    /// makes progress (`stuck`, `slice-dropped`).
    pub resume: Option<String>,
    /// Ready milestone (RFC-86 D22 / RFC-86a D7): every in-scope
    /// slice is refined with zero open **and** zero deferred
    /// findings. Deferrals never contribute; debt-carrying plans
    /// reach build via Authorized only. Never an `approved` rung.
    pub ready: bool,
    /// Authorized milestone (RFC-86 D22): a covering
    /// `plan.execute.started` epoch exists. Distinct from Ready even
    /// when the plan is clean. Never named `approved`.
    pub authorized: bool,
    /// Deferred-gap debt counts with conflicts broken out (RFC-86a
    /// D7). Debt never parks the loop; it surfaces here and at the
    /// change boundaries.
    pub debt: DebtCounts,
    /// Stop classification, populated when [`Self::action`] is
    /// [`NextActionKind::Stop`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopBody>,
    /// Publication milestone (RFC-95 D11): per-member materialized
    /// state and the next operator Git step. Empty before any
    /// in-scope entry exists.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub publication: Vec<PublicationMemberBody>,
    /// Typed gap inventory for in-scope slices (RFC-86 Gaps / D18 /
    /// D19 / D24). Same projection as `emery plan gaps`.
    pub gaps: GapsBody,
}

/// Closed per-member publication state on
/// [`PublicationMemberBody::state`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum PublicationMemberState {
    /// In-scope entries for this target are still pending or
    /// in-progress.
    AwaitingMerges,
    /// An unacknowledged postflight failure blocks materialize.
    Blocked,
    /// Every in-scope entry merged; `emery plan execute` materializes
    /// the worktree.
    Ready,
    /// The publication worktree carries the accepted CID.
    Materialized,
}

/// One publication member row on [`StatusBody::publication`].
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PublicationMemberBody {
    /// Target key.
    pub target: String,
    /// Closed member state.
    pub state: PublicationMemberState,
    /// Publication branch (`change/<plan>`), once materialized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    /// Node-local worktree path (observation only), once materialized.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worktree: Option<String>,
    /// Next operator step for this member.
    pub next: String,
}

/// Stop-conditions drained string: `drained — run /emery:finalize <name>`.
#[must_use]
pub fn drained_line(plan_name: &str) -> String {
    format!("drained \u{2014} run /emery:finalize {plan_name}")
}

/// Text rendering for `plan status`: a plan/entries header, then the
/// next-action line. Stops render the stop-conditions block shape
/// (`stop: <reason>` + indented context + `hint:`); drained renders
/// the literal stop-conditions drained string.
impl crate::handler::Render for StatusBody {
    fn render(&self, w: &mut dyn std::io::Write) -> std::io::Result<()> {
        writeln!(w, "plan: {}", self.plan)?;
        writeln!(
            w,
            "entries: {} done / {} in-progress / {} pending",
            self.counts.done, self.counts.in_progress, self.counts.pending
        )?;
        writeln!(w, "ready: {}  authorized: {}", self.ready, self.authorized)?;
        if !self.debt.is_empty() {
            writeln!(w, "debt: {}", self.debt)?;
        }
        match (self.action, &self.stop) {
            (NextActionKind::Drained, _) => writeln!(w, "{}", drained_line(&self.plan))?,
            (NextActionKind::Stop, Some(stop)) => {
                writeln!(w, "stop: {}", stop.reason)?;
                if let Some(slice) = &self.slice {
                    writeln!(w, "  slice: {slice}")?;
                    writeln!(w, "  target: {}", self.target.as_deref().unwrap_or("-"))?;
                }
                if let Some(detail) = &stop.detail {
                    writeln!(w, "  detail: {detail}")?;
                }
                writeln!(w, "hint: {}", stop.hint)?;
            }
            _ => writeln!(w, "next-action: {}", self.next_action)?,
        }
        if self.action != NextActionKind::Drained
            && let Some(resume) = &self.resume
        {
            writeln!(w, "resume: {resume}")?;
        }
        if !self.publication.is_empty() {
            writeln!(w, "publication:")?;
            for member in &self.publication {
                writeln!(w, "  {}: {} — {}", member.target, member.state, member.next)?;
            }
        }
        if !self.gaps.is_empty() {
            writeln!(w)?;
            self.gaps.render(w)?;
        }
        Ok(())
    }
}

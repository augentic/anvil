//! Read-only `emery plan status` projection.
//!
//! [`plan_status_body`] projects plan topology, slice artifacts, and
//! the fact union into a deterministic `next-action` —
//! `refine|build|merge <slice>`, `review-gaps`, `stop <reason>`, or
//! `drained` — so the execute loop renders the dispatch instead of
//! deriving it (RFC-86 D2 / D11). Also projects Ready (clean-gap) and
//! Authorized (covering `plan.execute.started`) milestones — never
//! an `approved` rung (D22 / D26). Writes nothing.
//!
//! This module owns the wire types; the per-entry decision kernel
//! lives in the shared `execution` projection and the body assembly
//! in `project`.

use serde::Serialize;

use super::gaps::GapsBody;

mod project;

pub use project::plan_status_body;

/// Closed next-action verb set on [`StatusBody::action`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum NextActionKind {
    /// Run `/emery:refine` for [`StatusBody::slice`].
    Refine,
    /// Run `/emery:build` for [`StatusBody::slice`].
    Build,
    /// Run `/emery:merge` for [`StatusBody::slice`].
    Merge,
    /// In-scope slices are refined but the clean gap policy fails —
    /// close conflicts / unknowns, or start execute with per-req
    /// `--waive` for unknowns (RFC-86 D22). Not an execute-loop
    /// phase; the loop maps this to build until the gap gate lands.
    ReviewGaps,
    /// Halt the loop; [`StatusBody::stop`] carries the reason.
    Stop,
    /// No pending or in-progress entries remain — the only clean exit.
    Drained,
}

/// Closed slice-loop step set for the re-entry fields
/// ([`StatusBody::current_step`] / [`StatusBody::last_completed`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, strum::Display)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum LoopStep {
    /// The refine phase (`/emery:refine`).
    Refine,
    /// The build phase (`/emery:build`).
    Build,
    /// The merge phase (`/emery:merge`, including the per-entry `done` stamp).
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
}

impl StopReason {
    /// Operator hint rendered under the stop block — one line, aligned
    /// with the stop-conditions reference's re-entry contract.
    #[must_use]
    pub const fn hint(self) -> &'static str {
        match self {
            Self::RefineFailed => {
                "Fix the failure, then retry /emery:refine for the slice. The plan entry stays \
                 in-progress."
            }
            Self::BuildFailed => {
                "Fix the failure, then retry /emery:build for the slice. The plan entry stays \
                 in-progress."
            }
            Self::MergeConflict => {
                "Resolve the baseline conflict (or drop the slice), then retry /emery:merge. The \
                 plan entry stays in-progress until the merge lands."
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
                "The slice is merged but the entry is still in-progress; re-run /emery:merge \
                 (or emery plan execute) for the slice — the merge re-entry stamps the missing \
                 done."
            }
            Self::Stuck => {
                "Remaining entries wait on unmet dependencies; complete or amend the blocking \
                 entries."
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
    /// `review-gaps`, `stop <reason>`, or `drained`.
    pub next_action: String,
    /// Closed verb behind [`Self::next_action`].
    pub action: NextActionKind,
    /// Slice the action targets; `None` on `stop`-without-slice,
    /// `review-gaps`, and `drained`.
    pub slice: Option<String>,
    /// Bound project of the targeted entry, when set.
    pub project: Option<String>,
    /// Step the targeted slice is currently at — the awaited
    /// phase, including a phase the loop is stopped on. `None` when no
    /// slice is targeted (`stuck`, `slice-dropped`, `review-gaps`,
    /// `drained`).
    pub current_step: Option<LoopStep>,
    /// Most recent step the targeted slice completed, from artifacts
    /// and success facts (`refined` → `refine`, `built` → `build`, a
    /// landed merge → `merge`). `None` before the first phase
    /// completes or when no slice is targeted.
    pub last_completed: Option<LoopStep>,
    /// Next valid resume point as a literal command — the phase
    /// skill for dispatches and retryable stops, `emery plan execute`
    /// / `/emery:execute` after author or when Ready (D26),
    /// `emery plan execute --waive…` when skipping Ready with open
    /// unknowns, `/emery:finalize` on drained. `None` when no single
    /// command makes progress (`stuck`, `slice-dropped`).
    pub resume: Option<String>,
    /// Ready milestone (RFC-86 D22): every in-scope slice is refined
    /// and the clean gap policy passes (no conflicts; zero open
    /// unknowns). Waivers never contribute. Never an `approved` rung.
    pub ready: bool,
    /// Authorized milestone (RFC-86 D22): a covering
    /// `plan.execute.started` epoch exists. Distinct from Ready even
    /// when the waive list is empty. The execute writer that appends
    /// the fact lands in a later session — until then this stays
    /// `false` unless a fixture stamps the event. Never named
    /// `approved`.
    pub authorized: bool,
    /// Stop classification, populated when [`Self::action`] is
    /// [`NextActionKind::Stop`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<StopBody>,
    /// Typed gap inventory for in-scope slices (RFC-86 Gaps / D18 /
    /// D19 / D24). Same projection as `emery plan gaps`.
    pub gaps: GapsBody,
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
        match (self.action, &self.stop) {
            (NextActionKind::Drained, _) => writeln!(w, "{}", drained_line(&self.plan))?,
            (NextActionKind::Stop, Some(stop)) => {
                writeln!(w, "stop: {}", stop.reason)?;
                if let Some(slice) = &self.slice {
                    writeln!(w, "  slice: {slice}")?;
                    writeln!(w, "  project: {}", self.project.as_deref().unwrap_or("-"))?;
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
        if !self.gaps.is_empty() {
            writeln!(w)?;
            self.gaps.render(w)?;
        }
        Ok(())
    }
}

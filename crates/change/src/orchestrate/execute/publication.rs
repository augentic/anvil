//! Publication reconcile inside the execute loop (RFC-95 D11): one
//! `worktree.export` call plus the deduped fact per pending member.
//! Never opens an epoch — authorization is the fact predicate.

use error::Error;
use jiff::Timestamp;
use project::handler::ExecutionPaths;
use project::journal::{self, Event, EventKind};
use project::plan::{Plan, StopReason, publication};
use project::seam::{Worktree, WorktreeError, WorktreeRequest};

/// How one reconcile pass ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reconciled {
    /// No member remains pending — every one carries its fact.
    Clean,
    /// A member's export refused; the loop stops on the typed reason.
    Stopped {
        /// `publication-worktree-dirty` or `publication-provision-failed`.
        reason: StopReason,
        /// The member and the closed refusal row.
        detail: String,
    },
}

/// Materialize every pending publication member (the D11 predicate:
/// complete, unblocked, accepted CID present, no covering fact),
/// journaling one `plan.publication.materialized` per export.
///
/// # Errors
///
/// Plan/journal I/O failures; export refusals return as
/// [`Reconciled::Stopped`], not `Err`.
pub async fn reconcile<W: Worktree>(
    worktree: &W, paths: &ExecutionPaths, now: Timestamp,
) -> Result<Reconciled, Error> {
    let layout = paths.layout();
    let plan = Plan::load(&layout.plan_path())?;
    let events = project::plan::collect_events(layout)?;
    let members = publication::members(&plan, layout, &events)?;
    // The in-place placement exists only for a single-member set on a
    // non-detached anchoring (D11).
    let allow_in_place = !paths.is_detached() && members.len() == 1;
    for member in members.iter().filter(|member| member.pending()) {
        let Some(cid) = member.accepted.clone() else {
            continue;
        };
        let branch = format!("change/{}", plan.name);
        let request = WorktreeRequest {
            repository: member.repository.clone(),
            parent_revision: member.parent_revision.clone(),
            branch: branch.clone(),
            cid: cid.clone(),
            plan: plan.name.to_string(),
            target: member.target.clone(),
            allow_in_place,
        };
        match worktree.export(request).await {
            Ok((worktree_path, state)) => {
                tracing::info!(
                    "publication worktree for {} {state:?} at {worktree_path}",
                    member.target
                );
                let event = Event::new(
                    now,
                    EventKind::PublicationMaterialized {
                        plan_name: plan.name.clone(),
                        plan_digest: Plan::file_digest(layout)?,
                        target: member.target.clone(),
                        parent_revision: member.parent_revision.clone(),
                        cid,
                        worktree_path,
                        branch,
                    },
                );
                journal::append_one(layout, &event)?;
            }
            Err(err) => {
                let reason = match err {
                    WorktreeError::Dirty => StopReason::PublicationWorktreeDirty,
                    _ => StopReason::PublicationProvision,
                };
                return Ok(Reconciled::Stopped {
                    reason,
                    detail: format!("target `{}`: {err}", member.target),
                });
            }
        }
    }
    Ok(Reconciled::Clean)
}

//! The merge orchestrator: target merge gates around the deterministic
//! core merge. The wave-commit fact is the accepted-CID transition.

use std::path::{Path, PathBuf};

use error::Error;
use jiff::Timestamp;
use project::build_record::BuildRecord;
use project::config::{Layout, ProjectConfig};
use project::journal::{self, DeferredMember, Event, EventKind, FactEpochRef, IdentityMap};
use project::name::SliceName;
use project::plan::{Plan, Status, collect_events, project_ladders};
use project::seam::{self, MergePhase, Target, Workspaces};
use project::snapshot::SnapshotId;
use project::wave::{Member, Wave};

use crate::merge::{
    MergeCommit, PreviewEntry, artifact_classes, debt, identity, slice as slice_merge,
};

mod gate;

use gate::{enforce_gate, fetch_gate_report, persist_gate_report, run_gate};

/// The result of a completed guest [`merge`].
#[derive(Debug, Clone)]
pub struct MergeOutcome {
    /// The 3-way merged spec/composition entries.
    pub merged: Vec<PreviewEntry>,
    /// `DEC-NNNN` ids promoted into the Decision Record catalogue.
    pub decisions: Vec<String>,
    /// Where the slice's working directory was archived.
    pub archive_path: PathBuf,
}

/// Context loaded once for wave commit + gates.
struct WaveCommit {
    /// Loaded and revalidated wave manifest.
    wave: Wave,
    /// Content digest of the wave (`sha256:…`).
    digest: String,
    /// Member records in stable wave order.
    members: Vec<(Member, BuildRecord)>,
}

/// Merge one built slice (the execute loop's merge phase).
///
/// Resolves the named slice's frozen wave, refuses until every member
/// result is present, prepares a writable workspace from the composed
/// member-result, runs per-member preflight, folds every delta inside
/// that workspace, captures the candidate CID, appends
/// `target.merge.wave-committed`, archives, then per-member postflight.
/// A crash before the commit fact leaves the prior accepted CID
/// authoritative; a crash after it is a resumable postflight stop.
///
/// # Errors
///
/// Completion-gate and preflight failures (slice not built / not
/// claimed in-progress, `target-merge-preflight-failed`), the terminal
/// `target-merge-postflight-failed`, plus commit and archive I/O
/// failures. Failures before `target.merge.wave-committed` leave no
/// merged projection.
#[tracing::instrument(
    name = "slice.merge",
    skip_all,
    fields(slice = %slice, target = tracing::field::Empty)
)]
pub async fn merge<T: Target + Workspaces>(
    targets: &T, layout: Layout<'_>, now: Timestamp, slice: &str, allow_composition_replace: bool,
) -> Result<MergeOutcome, Error> {
    tracing::info!("merge started");
    preflight_completion(layout, slice)?;
    if layout.plan_path().exists()
        && let Some(digest) =
            project::plan::author_overlap(layout, &Plan::load(&layout.plan_path())?)?
    {
        return Err(Error::validation_failed(
            "plan-ownership-overlap",
            "runtime ownership overlap writes an inert proposal",
            format!(
                "apply with `emery plan amend --proposal {digest}` after quiescing affected work"
            ),
        ));
    }
    if let Some(outcome) = already_complete(layout, slice) {
        tracing::info!("merge completed: commit and postflight already settled");
        return Ok(outcome);
    }
    let slice_dir = layout.slice_dir(slice);
    let target = project::target_policy::resumed(layout, slice)?;
    tracing::Span::current().record("target", target.as_str());
    let id =
        project::adapter::RoutedId::recorded(project::adapter::Axis::Target, &target).to_string();
    let commit = load_wave_commit(layout, slice, &slice_dir)?;

    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceMergeStarted {
            slice_name: slice.into(),
        },
        "slice.merge",
    );

    if wave_committed(layout, &commit.digest) {
        return resume_postflight(targets, layout, now, slice, &id, &commit).await;
    }

    let composed = composed_result(&commit)?;
    let view = journal_on_failure(
        layout,
        now,
        slice,
        prepare_workspace(targets, slice, &composed, true).await,
    )?;
    let run =
        gated(targets, layout, now, slice, &id, allow_composition_replace, &view, &commit).await;
    if let Err(err) = targets.discard(view.id.clone()).await {
        tracing::warn!(workspace = %view.id, "merge workspace discard failed: {err}");
    }
    let outcome = run?;

    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceMergeSucceeded {
            slice_name: slice.into(),
        },
        "slice.merge",
    );
    tracing::info!(decisions = outcome.decisions.len(), "merge completed");
    Ok(outcome)
}

/// Preflight → identity + fold inside the workspace → capture →
/// wave-committed → archive → postflight.
#[expect(
    clippy::too_many_arguments,
    reason = "internal merge kernel bracketed by the workspace lifecycle; callers use `merge`"
)]
async fn gated<T: Target + Workspaces>(
    targets: &T, layout: Layout<'_>, now: Timestamp, slice: &str, id: &str,
    allow_composition_replace: bool, view: &seam::Workspace, commit: &WaveCommit,
) -> Result<MergeOutcome, Error> {
    let product = Path::new(&view.root);
    let specs_dir = Layout::new(product).specs_dir();

    for (member, _) in &commit.members {
        let name = member.slice.as_str();
        let preflight = run_gate(targets, id, name, MergePhase::Preflight, view).await;
        let preflight = journal_on_failure(layout, now, slice, preflight)?;
        persist_gate_report(
            &layout.slice_dir(name).join("merge"),
            MergePhase::Preflight,
            &preflight,
        )?;
    }

    let mut identity_maps = Vec::new();
    let mut deferred = Vec::new();
    let mut folded: Vec<(SliceName, MergeCommit)> = Vec::new();
    for (member, _) in &commit.members {
        let name = member.slice.as_str();
        let slice_dir = layout.slice_dir(name);
        let maps =
            journal_on_failure(layout, now, slice, identity::finalize(&specs_dir, &slice_dir))?;
        identity_maps.extend(maps);
        let slice_debt = journal_on_failure(layout, now, slice, debt::carried(layout, name))?;
        journal_on_failure(layout, now, slice, debt::annotate(&slice_dir, &slice_debt))?;
        deferred.extend(slice_debt.rows.iter().map(|row| DeferredMember {
            req: row.req.clone(),
            status: row.status,
            requirement_digest: row.requirement_digest.clone(),
        }));

        let classes = artifact_classes(product, &slice_dir);
        let outcome = journal_on_failure(
            layout,
            now,
            slice,
            slice_merge::commit(&slice_dir, product, &classes, now, allow_composition_replace),
        )?;
        folded.push((member.slice.clone(), outcome));
    }

    let captured =
        journal_on_failure(layout, now, slice, capture_result(targets, slice, &view.id).await)?;
    let baseline = project::plan::dir_cid(&specs_dir)?;
    emit_wave_committed(
        layout,
        now,
        commit,
        &identity_maps,
        &deferred,
        &captured.result,
        baseline,
    )?;

    let mut named_archive = None;
    let mut named_fold = None;
    for (name, outcome) in &folded {
        let archive_path = journal_on_failure(
            layout,
            now,
            slice,
            archive_member(layout, now, name.as_str(), outcome),
        )?;
        if name.as_str() == slice {
            named_archive = Some(archive_path);
            named_fold = Some(outcome.clone());
        }
    }
    let archive_path = named_archive.ok_or_else(|| Error::Diag {
        code: "target-wave-member-mismatch",
        detail: format!(
            "wave for target `{}` does not name `{slice}` as a member",
            commit.wave.target
        ),
    })?;
    postflight_members(targets, layout, now, slice, id, commit, view).await?;

    let fold = named_fold.unwrap_or_else(|| MergeCommit {
        specs: vec![],
        decisions: vec![],
    });
    Ok(MergeOutcome {
        merged: fold.specs,
        decisions: fold.decisions,
        archive_path,
    })
}

/// Resume postflight after a commit fact: prepare a view of the
/// committed result, archive any member still in `slices/`, and run
/// remaining postflight gates.
async fn resume_postflight<T: Target + Workspaces>(
    targets: &T, layout: Layout<'_>, now: Timestamp, slice: &str, id: &str, commit: &WaveCommit,
) -> Result<MergeOutcome, Error> {
    let result = committed_result(layout, &commit.digest).ok_or_else(|| Error::Diag {
        code: "target-merge-resume-missing-result",
        detail: format!(
            "wave `{}` has a commit fact but no result CID; cannot resume postflight",
            commit.digest
        ),
    })?;
    let view = prepare_workspace(targets, slice, &result, false).await?;
    let run = resume_after_commit(targets, layout, now, slice, id, commit, &view).await;
    if let Err(err) = targets.discard(view.id.clone()).await {
        tracing::warn!(workspace = %view.id, "merge resume discard failed: {err}");
    }
    let outcome = run?;
    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceMergeSucceeded {
            slice_name: slice.into(),
        },
        "slice.merge",
    );
    Ok(outcome)
}

async fn resume_after_commit<T: Target>(
    targets: &T, layout: Layout<'_>, now: Timestamp, slice: &str, id: &str, commit: &WaveCommit,
    view: &seam::Workspace,
) -> Result<MergeOutcome, Error> {
    for (member, _) in &commit.members {
        let name = member.slice.as_str();
        let slice_dir = layout.slice_dir(name);
        if slice_dir.exists() {
            let empty = MergeCommit {
                specs: vec![],
                decisions: vec![],
            };
            journal_on_failure(layout, now, slice, archive_member(layout, now, name, &empty))?;
        }
    }
    postflight_members(targets, layout, now, slice, id, commit, view).await?;
    let archive_path = project::refinement::latest_archive(&layout.archive_dir(), slice)
        .ok_or_else(|| Error::Diag {
            code: "merge-archive-failed",
            detail: format!("slice `{slice}` has no archive after wave commit"),
        })?;
    Ok(MergeOutcome {
        merged: vec![],
        decisions: vec![],
        archive_path,
    })
}

/// Per-member postflight in stable order; resumes at the first missing
/// report. Aggregates every failed member onto one fact.
async fn postflight_members<T: Target>(
    targets: &T, layout: Layout<'_>, now: Timestamp, slice: &str, id: &str, commit: &WaveCommit,
    view: &seam::Workspace,
) -> Result<(), Error> {
    let mut failed: Vec<SliceName> = Vec::new();
    let mut last_err: Option<Error> = None;
    for (member, _) in &commit.members {
        let name = member.slice.as_str();
        let Some(archive) = project::refinement::latest_archive(&layout.archive_dir(), name) else {
            failed.push(member.slice.clone());
            last_err = Some(Error::Diag {
                code: "merge-archive-failed",
                detail: format!("slice `{name}` has no archive for postflight"),
            });
            continue;
        };
        let report_dir = archive.join("merge");
        if report_dir.join("postflight.yaml").is_file() {
            continue;
        }
        match fetch_gate_report(targets, id, name, MergePhase::Postflight, view).await {
            Ok(report) => {
                let persist_err =
                    persist_gate_report(&report_dir, MergePhase::Postflight, &report).err();
                if let Err(err) = enforce_gate(&report, MergePhase::Postflight, name) {
                    failed.push(member.slice.clone());
                    last_err = Some(err);
                }
                if let Some(err) = persist_err {
                    failed.push(member.slice.clone());
                    last_err = Some(err);
                }
            }
            Err(err) => {
                failed.push(member.slice.clone());
                last_err = Some(err);
            }
        }
    }
    if let Some(err) = last_err {
        return postflight_terminal(layout, now, commit, failed, &err);
    }
    journal::emit_best_effort(
        layout,
        now,
        EventKind::TargetMergeWaveSucceeded {
            target: commit.wave.target.clone(),
            digest: commit.digest.clone(),
            slice_name: slice.into(),
        },
        "slice.merge",
    );
    Ok(())
}

/// Resolve the named slice's wave, load every member result, and
/// revalidate the named slice's record against the manifest.
fn load_wave_commit(
    layout: Layout<'_>, slice: &str, slice_dir: &Path,
) -> Result<WaveCommit, Error> {
    let opened = opened_wave_digest(layout, slice)?;
    let record = BuildRecord::load_for_wave(slice_dir, &opened)?;
    let config = ProjectConfig::load(layout.project_dir())?;
    let wave = Wave::load_for_merge(layout, &config.name, slice, &record)?;
    let digest = wave.digest()?.as_str().to_string();
    let members = wave.load_member_records(layout)?;
    Ok(WaveCommit {
        wave,
        digest,
        members,
    })
}

/// The newest `target.wave.opened` fact naming `slice` in the ordered
/// event union — the wave the build phase authorized (RFC-86 D9).
fn opened_wave_digest(layout: Layout<'_>, slice: &str) -> Result<SnapshotId, Error> {
    let events = collect_events(layout)?;
    let digest = events
        .iter()
        .rev()
        .find_map(|event| match &event.kind {
            EventKind::TargetWaveOpened {
                digest, slice_name, ..
            } if slice_name.as_str() == slice => Some(digest.clone()),
            _ => None,
        })
        .ok_or_else(|| {
            Error::validation_failed(
                "target-wave-not-opened",
                "a merge resolves its build record through the slice's `target.wave.opened` fact",
                format!(
                    "no `target.wave.opened` fact names slice `{slice}`; re-run \
                     `emery plan execute` so the build phase opens a wave before merging"
                ),
            )
        })?;
    SnapshotId::parse(&digest)
}

/// This cut's composed member-result: the sole `BuildRecord.result`,
/// loaded through the member list.
fn composed_result(commit: &WaveCommit) -> Result<SnapshotId, Error> {
    commit.wave.enforce_one_member()?;
    commit.members.iter().map(|(_, record)| record.result.clone()).next().ok_or_else(|| {
        Error::Diag {
            code: "target-wave-member-count",
            detail: format!(
                "target wave for `{}` must have exactly one member; found 0",
                commit.wave.target
            ),
        }
    })
}

async fn prepare_workspace(
    workspaces: &impl Workspaces, slice: &str, base: &SnapshotId, writable: bool,
) -> Result<seam::Workspace, Error> {
    workspaces.prepare(base.clone(), writable).await.map_err(|err| Error::Diag {
        code: "target-merge-workspace-failed",
        detail: format!(
            "preparing the merge workspace for slice `{slice}` failed (base `{base}`): {err}"
        ),
    })
}

async fn capture_result(
    workspaces: &impl Workspaces, slice: &str, id: &str,
) -> Result<project::snapshot::CodePatch, Error> {
    workspaces.capture(id.to_string()).await.map_err(|err| Error::Diag {
        code: "target-merge-workspace-failed",
        detail: format!("capturing the merge workspace for slice `{slice}` failed: {err}"),
    })
}

fn emit_wave_committed(
    layout: Layout<'_>, now: Timestamp, commit: &WaveCommit, maps: &[IdentityMap],
    deferred: &[DeferredMember], result: &SnapshotId, baseline: SnapshotId,
) -> Result<(), Error> {
    let auth = &commit.wave.build_authorization;
    let members: Vec<SliceName> = commit.wave.members.iter().map(|m| m.slice.clone()).collect();
    journal::append_one(
        layout,
        &Event::new(
            now,
            EventKind::TargetMergeWaveCommitted {
                target: commit.wave.target.clone(),
                digest: commit.digest.clone(),
                members,
                base: commit.wave.base.clone(),
                result: result.clone(),
                commit_authorization: FactEpochRef {
                    writer: auth.writer.clone(),
                    sequence: auth.sequence,
                },
                identity_maps: maps.to_vec(),
                baseline: Some(baseline),
                deferred: deferred.to_vec(),
            },
        ),
    )
}

fn postflight_terminal(
    layout: Layout<'_>, now: Timestamp, commit: &WaveCommit, failed: Vec<SliceName>, err: &Error,
) -> Result<(), Error> {
    let members = if failed.is_empty() {
        commit.wave.members.iter().map(|m| m.slice.clone()).collect()
    } else {
        failed
    };
    let named = members.iter().map(SliceName::as_str).collect::<Vec<_>>().join(", ");
    let detail = format!(
        "target postflight merge gate failed for member(s) `{named}` after the wave \
         committed — the accepted CID and `target.merge.wave-committed` fact stand \
         (non-rollback); inspect the archive `merge/postflight.yaml` when present \
         and land a follow-up slice: {err}"
    );
    let event = Event::new(
        now,
        EventKind::MergeWavePostflightFailed {
            target: commit.wave.target.clone(),
            digest: commit.digest.clone(),
            members,
            reason: err.variant_str().into_owned(),
        },
    );
    if let Err(journal_err) = journal::append_one(layout, &event) {
        return Err(Error::Diag {
            code: "target-merge-postflight-failed",
            detail: format!("{detail}; also failed to journal postflight debt: {journal_err}"),
        });
    }
    Err(Error::Diag {
        code: "target-merge-postflight-failed",
        detail,
    })
}

fn journal_on_failure<V>(
    layout: Layout<'_>, now: Timestamp, slice: &str, result: Result<V, Error>,
) -> Result<V, Error> {
    if let Err(err) = &result {
        journal::emit_best_effort(
            layout,
            now,
            EventKind::SliceMergeFailed {
                slice_name: slice.into(),
                reason: err.variant_str().into_owned(),
            },
            "slice.merge",
        );
    }
    result
}

/// Commit fact exists and postflight has settled (succeeded or the
/// aggregate failed fact). Re-entry is a no-op.
fn already_complete(layout: Layout<'_>, slice: &str) -> Option<MergeOutcome> {
    let events = collect_events(layout).ok()?;
    let digest = events.iter().rev().find_map(|event| match &event.kind {
        EventKind::TargetMergeWaveCommitted { digest, members, .. }
            if members.iter().any(|m| m.as_str() == slice) =>
        {
            Some(digest.clone())
        }
        _ => None,
    })?;
    let settled = events.iter().any(|event| match &event.kind {
        EventKind::TargetMergeWaveSucceeded { digest: d, .. } if *d == digest => true,
        EventKind::MergeWavePostflightFailed { digest: d, .. } if *d == digest => true,
        _ => false,
    });
    if !settled {
        return None;
    }
    let archive_path = project::refinement::latest_archive(&layout.archive_dir(), slice)?;
    Some(MergeOutcome {
        merged: vec![],
        decisions: vec![],
        archive_path,
    })
}

fn wave_committed(layout: Layout<'_>, digest: &str) -> bool {
    collect_events(layout).ok().is_some_and(|events| {
        events.iter().any(|event| match &event.kind {
            EventKind::TargetMergeWaveCommitted { digest: d, .. } => d == digest,
            _ => false,
        })
    })
}

fn committed_result(layout: Layout<'_>, digest: &str) -> Option<SnapshotId> {
    collect_events(layout).ok()?.into_iter().rev().find_map(|event| match event.kind {
        EventKind::TargetMergeWaveCommitted {
            digest: d, result, ..
        } if d == digest => Some(result),
        _ => None,
    })
}

fn preflight_completion(layout: Layout<'_>, slice: &str) -> Result<(), Error> {
    if !layout.plan_path().exists() {
        return Ok(());
    }
    let plan = Plan::load(&layout.plan_path())?;
    let Some(entry) = plan.entries.iter().find(|e| e.name == slice) else {
        return Err(plan.entry_not_found(slice));
    };
    let events = collect_events(layout)?;
    let ladders = project_ladders(&plan, &events);
    let status = ladders.get(&entry.name).copied().unwrap_or(Status::Pending);
    if status != Status::InProgress {
        return Err(Error::validation_failed(
            "slice-merge-entry-not-in-progress",
            "a plan-owned merge requires a projected `in-progress` entry",
            format!(
                "plan entry `{slice}` projects `{status}`; re-run `emery plan execute` — the \
                 loop claims the entry before merging"
            ),
        ));
    }
    Ok(())
}

fn archive_member(
    layout: Layout<'_>, now: Timestamp, slice: &str, folded: &MergeCommit,
) -> Result<PathBuf, Error> {
    let slice_dir = layout.slice_dir(slice);
    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceMergeCommitSkipped {
            slice_name: slice.into(),
        },
        "slice.merge",
    );
    let archive_path =
        crate::actions::archive(&slice_dir, &layout.archive_dir(), now).map_err(|err| {
            Error::Diag {
                code: "merge-archive-failed",
                detail: format!("archive move failed: {err}"),
            }
        })?;
    emit_archive_created(layout, now, slice, folded);
    Ok(archive_path)
}

fn emit_archive_created(layout: Layout<'_>, now: Timestamp, slice: &str, merged: &MergeCommit) {
    let touched_specs: Vec<String> = merged.specs.iter().map(|e| e.name.clone()).collect();
    let outcome_summary = if merged.specs.is_empty() {
        "no baseline specs touched".to_string()
    } else {
        merged
            .specs
            .iter()
            .map(|e| {
                format!("{}: {}", e.name, crate::merge::summarise_operations(&e.result.operations))
            })
            .collect::<Vec<_>>()
            .join("; ")
    };
    journal::emit_best_effort(
        layout,
        now,
        EventKind::SliceArchiveCreated {
            slice_name: slice.into(),
            touched_specs,
            outcome_summary,
            merge_sha: None,
            decisions: merged.decisions.clone(),
        },
        "slice.archive.created",
    );
}

//! `plan archive` — move the current plan into the archive and run
//! the change-scoped snapshot sweep.

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use artifacts::spec::provenance::RequirementStatus;
use error::Error;
use jiff::Timestamp;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::build_record::BuildRecord;
use project::handler::{Anchor, Ctx, Render};
use project::journal::{DeferralOrigin, Event, EventKind};
use project::plan::{Plan, collect_events};
use project::seam::Workspaces;
use project::snapshot::SnapshotId;
use serde::{Deserialize, Serialize};
use slice::Base;

/// Wire input for `plan archive`.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArchiveInput {
    /// Archive even when the plan has pending or in-progress entries.
    #[serde(default)]
    pub force: bool,
}

/// `emery plan archive` — close the change.
///
/// Moves the current plan to
/// `.emery/archive/plans/<name>-<YYYYMMDD>.yaml`, then sweeps the
/// snapshot store: the archived change's pins stop being GC roots
/// (RFC-88 D2), so objects reachable only from archived slice trees
/// are deleted.
#[derive(Clone, Copy, Debug)]
pub struct Archive;

impl<P: Anchor + Workspaces> Operation<P> for Archive {
    type Error = project::handler::Error;
    type Input = ArchiveInput;
    type Output = ArchiveBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let layout = cx.layout();
        let plan_path = layout.plan_path();
        if !plan_path.exists() {
            return Err(Error::ArtifactNotFound {
                kind: "plan.yaml",
                path: plan_path,
            }
            .into());
        }
        let archive_dir = layout.archive_dir().join("plans");
        let brief_path = layout.change_brief_path();
        let plan = Plan::load(&plan_path)?;
        let plan_name = plan.name.to_string();

        // Carried-debt summary (RFC-86a D5/D9): read before the plan
        // moves; advisory only — archiving never blocks on debt.
        let debt = carried_debt(layout, &plan, cx.now())?;

        let (archived, archived_plans_dir) =
            Plan::archive(&plan_path, &brief_path, &archive_dir, input.force, cx.now())?;

        // Change-scoped collection: pins under the archive tree are
        // dead roots; pins under any slice tree still live (a forced
        // archive leaves unfinished slices in place) are kept.
        let dead = collect_pins(&layout.archive_dir())?;
        let live = collect_pins(&layout.slices_dir())?;
        let swept_objects =
            context.provider.sweep(dead, live).await.map_err(|err| Error::Diag {
                code: "snapshot-sweep-failed",
                detail: format!(
                    "plan `{plan_name}` archived, but the snapshot sweep failed: {err}"
                ),
            })?;

        Ok(ArchiveBody {
            archived,
            archived_plans_dir,
            swept_objects,
            debt,
            plan: ArchivedPlan { name: plan_name },
        })
    }
}

/// The debt the archived change carried into the baseline: each
/// committed wave's deferred member-set snapshot, joined back to its
/// covering `gap.deferred` fact for reason, origin, and age.
fn carried_debt(
    layout: project::config::Layout<'_>, plan: &Plan, now: Timestamp,
) -> Result<Vec<DebtRow>, Error> {
    let events = collect_events(layout)?;
    let deferrals = latest_deferrals(&events);
    let mut rows = Vec::new();
    for entry in &plan.entries {
        let Some(members) = latest_wave_deferred(&events, entry.name.as_str()) else {
            continue;
        };
        for member in members {
            // Every member was covered by a deferral fact at merge
            // time; a join miss (pruned or damaged journal) degrades
            // to a placeholder row rather than dropping the debt.
            let deferral = deferrals
                .get(&(entry.name.as_str().to_string(), member.requirement_digest.clone()))
                .map(|(reason, origin, deferred_at)| {
                    let age_days =
                        u64::try_from((now.as_second() - deferred_at.as_second()).max(0))
                            .unwrap_or(0)
                            / 86_400;
                    DebtDetail {
                        reason: reason.clone(),
                        origin: *origin,
                        deferred_at: *deferred_at,
                        age_days,
                    }
                });
            if deferral.is_none() {
                tracing::warn!(
                    "no covering gap.deferred fact for wave snapshot member {}/{} ({}); \
                     rendering the debt row without its provenance detail",
                    entry.name.as_str(),
                    member.req,
                    member.requirement_digest
                );
            }
            rows.push(DebtRow {
                slice: entry.name.as_str().to_string(),
                req: member.req.clone(),
                status: member.status,
                deferral,
            });
        }
    }
    Ok(rows)
}

/// Deferred member set of the newest `target.merge.wave-committed`
/// fact naming `slice`, or `None` when the slice never merged.
fn latest_wave_deferred<'e>(
    events: &'e [Event], slice: &str,
) -> Option<&'e [project::journal::DeferredMember]> {
    events.iter().rev().find_map(|event| match &event.kind {
        EventKind::TargetMergeWaveCommitted {
            slice_name, deferred, ..
        } if slice_name.as_str() == slice => Some(deferred.as_slice()),
        _ => None,
    })
}

/// Latest `gap.deferred` detail per `(slice, digest)`. Retractions are
/// deliberately not folded in: the wave snapshot is the authority that
/// the debt landed, and a post-merge retraction must not erase the
/// summary's reason.
fn latest_deferrals(
    events: &[Event],
) -> BTreeMap<(String, String), (String, DeferralOrigin, Timestamp)> {
    let mut latest = BTreeMap::new();
    for event in events {
        if let EventKind::GapDeferred {
            slice,
            requirement_digest,
            reason,
            origin,
            ..
        } = &event.kind
        {
            latest.insert(
                (slice.as_str().to_string(), requirement_digest.clone()),
                (reason.clone(), *origin, event.timestamp),
            );
        }
    }
    latest
}

/// Every snapshot pin recorded beneath one level of slice-shaped
/// directories under `root`: `base.yaml` pins plus each
/// `builds/<digest>.yaml` record's base and result snapshots. Roots
/// that never reached the store are skipped by the sweep itself.
fn collect_pins(root: &Path) -> Result<Vec<SnapshotId>, Error> {
    let mut pins = Vec::new();
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => return Ok(pins),
        Err(source) => {
            return Err(Error::Filesystem {
                op: "read_dir",
                path: root.to_path_buf(),
                source,
            });
        }
    };
    for entry in entries.filter_map(Result::ok) {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        if Base::path(&dir).is_file() {
            let base = Base::load(&dir)?;
            pins.extend(base.sources.into_values());
            pins.push(base.baseline_spec);
            pins.push(base.target_base);
        }
        for record in BuildRecord::load_all(&dir)? {
            pins.push(record.base);
            pins.push(record.result);
        }
    }
    Ok(pins)
}

/// Success envelope for `plan archive`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArchiveBody {
    /// Path of the archived plan file (serialised as its display string).
    pub archived: PathBuf,
    /// Path of the moved working directory, when one moved.
    pub archived_plans_dir: Option<PathBuf>,
    /// Snapshot objects deleted by the change-scoped sweep.
    pub swept_objects: usize,
    /// Debt the archived change carried into the baseline (RFC-86a
    /// D5/D9) — advisory; archiving never blocks on debt.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub debt: Vec<DebtRow>,
    /// The archived plan's identity.
    pub plan: ArchivedPlan,
}

/// One carried-debt row in the archive summary.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DebtRow {
    /// Slice that folded the row into the baseline.
    pub slice: String,
    /// Final baseline `REQ-NNN`.
    pub req: String,
    /// Typed gap status (`unknown` | `conflict`).
    pub status: RequirementStatus,
    /// The covering `gap.deferred` fact's detail. `None` when the
    /// fact join misses (a pruned or damaged journal) — the row is
    /// still debt, just without its provenance detail.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deferral: Option<DebtDetail>,
}

/// The covering deferral fact's detail on one carried-debt row.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DebtDetail {
    /// Covering deferral's reason.
    pub reason: String,
    /// Which surface dispositioned the requirement.
    pub origin: DeferralOrigin,
    /// When the covering fact was appended.
    #[serde(with = "project::serde_time::rfc3339")]
    pub deferred_at: Timestamp,
    /// Whole days between the deferral and the archive.
    pub age_days: u64,
}

/// The archived plan's identity.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ArchivedPlan {
    /// Plan name.
    pub name: String,
}

impl ArchiveBody {
    /// Render the carried-debt rows of one gap kind under `heading` —
    /// deferred conflicts and deferred unknowns get separate blocks
    /// (RFC-86a D6: a shipped-around contradiction is louder news).
    fn render_debt(
        &self, w: &mut dyn Write, status: RequirementStatus, heading: &str,
    ) -> std::io::Result<()> {
        let mut headed = false;
        for row in self.debt.iter().filter(|row| row.status == status) {
            if !headed {
                writeln!(w, "    {heading}")?;
                headed = true;
            }
            match &row.deferral {
                Some(detail) => {
                    let noun = if detail.age_days == 1 { "day" } else { "days" };
                    writeln!(
                        w,
                        "      {}/{} — {} ({}, {} {noun})",
                        row.slice, row.req, detail.reason, detail.origin, detail.age_days
                    )?;
                }
                None => writeln!(
                    w,
                    "      {}/{} — reason unavailable (no covering deferral fact)",
                    row.slice, row.req
                )?,
            }
        }
        Ok(())
    }
}

impl Render for ArchiveBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "archived plan `{}`", self.plan.name)?;
        writeln!(w, "  archived: {}", self.archived.display())?;
        if let Some(dir) = &self.archived_plans_dir {
            writeln!(w, "  working directory: {}", dir.display())?;
        }
        writeln!(w, "  swept snapshot objects: {}", self.swept_objects)?;
        if !self.debt.is_empty() {
            writeln!(w, "  carried debt ({} deferred):", self.debt.len())?;
            self.render_debt(w, RequirementStatus::Unknown, "unknown:")?;
            self.render_debt(w, RequirementStatus::Conflict, "conflict:")?;
        }
        Ok(())
    }
}

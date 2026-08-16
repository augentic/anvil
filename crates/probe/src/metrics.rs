//! RFC-96 D11: the coordination-cost report, projected after a
//! workflow case from journal facts, build records, and telemetry.
//! Cost stays `unknown` until RFC-92 usage facts land.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use anyhow::{Context as _, Result};
use jiff::{SignedDuration, Timestamp};
use project::build_record::BuildRecord;
use project::config::Layout;
use project::journal::{Event, EventKind};
use project::snapshot::SnapshotId;

/// One accepted-target row: the graded accepted CID plus the
/// requirement count and materialized tree size behind it.
#[derive(Debug)]
pub struct Accepted {
    /// Target key under `plan.yaml.targets`.
    pub target: String,
    /// The accepted CID grading materialized.
    pub cid: SnapshotId,
    /// Baseline requirements graded from that tree.
    pub requirements: usize,
    /// Total file bytes of the materialized accepted tree — the
    /// code-growth signal for a cap-comparison pair.
    pub bytes: u64,
}

/// The coordination-cost report (RFC-96 D11).
#[derive(Debug)]
pub struct Coordination {
    /// Effective operation cap the run used ([`project::pool::cap`]).
    pub cap: usize,
    /// Per-target accepted rows (requirements, CIDs, code growth).
    pub accepted: Vec<Accepted>,
    /// Wall-clock from `plan.execute.started` to the first
    /// `target.merge.wave-committed` — time to first accepted result.
    pub first_accepted: Option<SignedDuration>,
    /// `slice.build.started` fact count — every build attempt start.
    pub builds: usize,
    /// Builds beyond one per distinct slice — the rebuild count.
    pub rebuilds: usize,
    /// Waves opened per target (`target.wave.opened` facts).
    pub waves: BTreeMap<String, usize>,
    /// Touched-path heat across every build record: path → touches.
    pub heat: BTreeMap<String, usize>,
    /// Model requests per judgment leg — the planner-induced worker
    /// usage signal, from the case run's request telemetry.
    pub requests: BTreeMap<String, usize>,
    /// Reported provider cost. `None` renders `unknown`: RFC-92 usage
    /// facts have not landed and the report never fabricates cost.
    pub cost: Option<String>,
}

/// Project the coordination-cost report from the run's facts.
///
/// `requests` is the per-leg tally from the case's model telemetry;
/// `accepted` carries the rows [the grading][crate::grade] already
/// materialized.
///
/// # Errors
///
/// Returns slices-dir I/O failures and unreadable build records.
pub fn coordination(
    layout: Layout<'_>, events: &[Event], requests: BTreeMap<String, usize>,
    accepted: Vec<Accepted>,
) -> Result<Coordination> {
    let mut builds = 0_usize;
    let mut built_slices: BTreeSet<String> = BTreeSet::new();
    let mut waves: BTreeMap<String, usize> = BTreeMap::new();
    let mut execute_started: Option<Timestamp> = None;
    let mut first_commit: Option<Timestamp> = None;
    for event in events {
        match &event.kind {
            EventKind::SliceBuildStarted { slice_name } => {
                builds += 1;
                built_slices.insert(slice_name.to_string());
            }
            EventKind::TargetWaveOpened { target, .. } => {
                *waves.entry(target.clone()).or_default() += 1;
            }
            EventKind::PlanExecuteStarted { .. } => {
                execute_started = execute_started.or(Some(event.timestamp));
            }
            EventKind::TargetMergeWaveCommitted { .. } => {
                first_commit = first_commit.or(Some(event.timestamp));
            }
            _ => {}
        }
    }
    let first_accepted = match (execute_started, first_commit) {
        (Some(started), Some(committed)) => Some(committed.duration_since(started)),
        _ => None,
    };
    Ok(Coordination {
        cap: project::pool::cap(),
        accepted,
        first_accepted,
        builds,
        rebuilds: builds.saturating_sub(built_slices.len()),
        waves,
        heat: heat(&layout.slices_dir())?,
        requests,
        cost: None,
    })
}

/// Render the report to the run log, one line per signal.
pub fn render(report: &Coordination) {
    tracing::info!("coordination-cost report (cap {})", report.cap);
    for row in &report.accepted {
        tracing::info!(
            "  accepted target `{}`: {} requirement(s), {} byte(s), cid {}",
            row.target,
            row.requirements,
            row.bytes,
            row.cid
        );
    }
    if let Some(elapsed) = report.first_accepted {
        tracing::info!("  time to first accepted result: {elapsed:#}");
    } else {
        tracing::info!("  time to first accepted result: unknown (no commit fact)");
    }
    tracing::info!("  builds: {} started, {} rebuild(s)", report.builds, report.rebuilds);
    for (target, count) in &report.waves {
        tracing::info!("  waves for `{target}`: {count}");
    }
    let hot: Vec<String> = report
        .heat
        .iter()
        .filter(|(_, touches)| **touches > 1)
        .map(|(path, touches)| format!("{path} ×{touches}"))
        .collect();
    tracing::info!(
        "  touched paths: {} distinct{}",
        report.heat.len(),
        if hot.is_empty() { String::new() } else { format!(", hot: {}", hot.join(", ")) }
    );
    for (leg, count) in &report.requests {
        tracing::info!("  requests leg `{leg}`: {count}");
    }
    tracing::info!("  reported cost: {}", report.cost.as_deref().unwrap_or("unknown"));
}

/// Total file bytes under `root` — the code-growth measure over a
/// materialized accepted tree.
///
/// # Errors
///
/// Returns directory-walk I/O failures.
pub fn tree_bytes(root: &Path) -> Result<u64> {
    let mut total = 0;
    let entries = fs::read_dir(root).with_context(|| format!("reading tree {}", root.display()))?;
    for entry in entries {
        let entry = entry.context("tree entry")?;
        let kind = entry.file_type().context("tree entry type")?;
        if kind.is_dir() {
            total += tree_bytes(&entry.path())?;
        } else if kind.is_file() {
            total += entry.metadata().context("tree entry metadata")?.len();
        }
    }
    Ok(total)
}

// Touched-path heat from every persisted build record across every
// slice: how often each workspace-relative path was rewritten.
fn heat(slices_dir: &Path) -> Result<BTreeMap<String, usize>> {
    let mut heat = BTreeMap::new();
    let Ok(entries) = fs::read_dir(slices_dir) else {
        return Ok(heat);
    };
    for entry in entries {
        let slice = entry.context("slice dir entry")?.path();
        if !slice.is_dir() {
            continue;
        }
        for record in BuildRecord::load_all(&slice)
            .with_context(|| format!("build records under {}", slice.display()))?
        {
            for path in record.touched {
                *heat.entry(path).or_default() += 1;
            }
        }
    }
    Ok(heat)
}

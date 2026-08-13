//! The `emery system plan` orchestration (RFC-104 D8–D10).
//!
//! Validate the definition, run the initial-plan proposal judgment
//! when `target` is absent at load, reproject every architecture
//! view, and project each wave's canonical handoff. Re-running the
//! stage is resume — plan always reprojects from live files and never
//! overwrites an operator-owned named state or `migration.yaml`.

use std::collections::BTreeSet;
use std::path::Path;

use error::Error;
use omnia_guest::Model as ModelSeam;
use project::handler::ExecutionPaths;
use project::snapshot::SnapshotId;

use super::read_corpus;
use crate::coverage::Coverage;
use crate::judgment::propose;
use crate::layout::Layout;
use crate::migration::Migration;
use crate::model::Model;
use crate::scope::Scope;
use crate::{architecture, decision, handoff};

/// The completed run's accounting, projected by the operation body.
#[derive(Debug)]
pub struct PlanOutcome {
    /// The declared engagement identity (`scope.yaml.id`).
    pub id: String,
    /// True when this run was the initial architecture proposal.
    pub proposed: bool,
    /// The named states whose views were reprojected, in order.
    pub states: Vec<String>,
    /// One entry per projected wave handoff, in plan order.
    pub waves: Vec<WaveHandoff>,
}

/// One projected wave handoff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaveHandoff {
    /// The wave's id in `migration.yaml`.
    pub wave: String,
    /// The canonical handoff digest (its filename and identity).
    pub digest: SnapshotId,
}

/// Run the plan stage over the anchored definition home.
///
/// # Errors
///
/// Fail-closed loads (`system-scope-missing`,
/// `system-coverage-missing`, `system-model-missing` — survey has not
/// run, `decisions/` validation), `system-overlay-stale` when
/// identities or decision stamps require a re-survey, proposal
/// judgment failures, `system-handoff-unresolved` references, and
/// persist I/O failures.
pub async fn plan(seam: &impl ModelSeam, paths: &ExecutionPaths) -> Result<PlanOutcome, Error> {
    let root = paths.project_root();
    let layout = Layout::new(root);
    let scope = Scope::load(&layout.scope_path())?;
    let coverage = Coverage::load(&layout.coverage_path())?;
    let decisions = decision::load_all(&layout.decisions_dir())?;
    let mut model = match Model::load(&layout.system_path()) {
        Err(Error::Diag {
            code: "system-model-missing",
            ..
        }) => {
            return Err(Error::Diag {
                code: "system-model-missing",
                detail: format!(
                    "system.yaml not found at {} — run `emery system survey` first; plan \
                     projects from the surveyed model",
                    layout.system_path().display()
                ),
            });
        }
        loaded => loaded?,
    };

    // Initial architecture proposal: only when `target` is absent at
    // load, and never overwriting an existing named state or an
    // operator-owned migration.yaml.
    let proposed = if model.target.is_none() {
        propose_initial(seam, &layout, &coverage, &scope, &decisions, &mut model).await?;
        true
    } else {
        false
    };

    // Reproject every named state's views and prune views of states
    // that no longer exist, so a stale projection cannot survive.
    let mut views = vec![("as-is", &model.as_is)];
    if let Some(target) = &model.target {
        views.push(("target", target));
    }
    views.extend(model.transitions.iter().map(|(name, state)| (name.as_str(), state)));
    for (name, state) in &views {
        architecture::project(&layout, name, state)?;
    }
    let states: Vec<String> = views.iter().map(|(name, _)| (*name).to_string()).collect();
    prune_views(&layout, &states)?;
    architecture::validate(&layout, &model)?;

    // Project every candidate wave's canonical handoff; historical
    // handoffs are never deleted.
    let mut waves = Vec::new();
    match Migration::load(&layout.migration_path()) {
        Err(Error::Diag {
            code: "system-migration-missing",
            ..
        }) => {}
        Err(err) => return Err(err),
        Ok(migration) => {
            for wave in &migration.waves {
                let projected = handoff::project(
                    &layout, &scope, &coverage, &model, &migration, &decisions, wave,
                )?;
                handoff::write(&layout, &projected)?;
                waves.push(WaveHandoff {
                    wave: wave.id.clone(),
                    digest: projected.digest,
                });
            }
        }
    }

    Ok(PlanOutcome {
        id: scope.id,
        proposed,
        states,
        waves,
    })
}

/// Remove projection files whose stem is not a live named state.
fn prune_views(layout: &Layout<'_>, states: &[String]) -> Result<(), Error> {
    let live: BTreeSet<&str> = states.iter().map(String::as_str).collect();
    let dirs = [
        layout.architecture_dir(),
        layout.architecture_dir().join("transitions"),
        layout.diagrams_dir(),
    ];
    for dir in dirs {
        prune_dir(&dir, &live)?;
    }
    Ok(())
}

/// Remove one directory's orphan projection files.
fn prune_dir(dir: &Path, live: &BTreeSet<&str>) -> Result<(), Error> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(Error::Io(err)),
    };
    for entry in entries {
        let path = entry.map_err(Error::Io)?.path();
        let projected = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext, "md" | "svg" | "source"));
        let stem = path.file_stem().and_then(|stem| stem.to_str());
        if let (true, Some(stem)) = (projected, stem)
            && !live.contains(stem)
        {
            std::fs::remove_file(&path).map_err(Error::Io)?;
        }
    }
    Ok(())
}

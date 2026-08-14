//! Closed-plan verification chain shared by authoring and execute start.
//!
//! Fixture plans skip unrecorded legs; a recorded digest must match
//! the live retained revision.

use std::path::{Path, PathBuf};

use artifacts::leads::Leads;
use diagnostics::digest::sha256_hex;
use diagnostics::has_blocking;
use error::Error;

use super::decomposition::{self, Decomposition};
use super::discovery::Discovery;
use super::model::{DefinitionIdentity, Plan, TargetBinding};
use crate::config::Layout;
use crate::handler::ExecutionPaths;
use crate::journal::{Event, EventKind};
use crate::refinement::{self, Manifest};
use crate::snapshot::SnapshotId;

/// Verify the execute-start digest chain (D8).
///
/// # Errors
///
/// Typed `plan-*` diagnostics for each drifted or missing pin.
pub fn closed_plan(paths: &ExecutionPaths, plan: &Plan) -> Result<(), Error> {
    let layout = paths.layout();
    discovery(&layout, plan)?;
    leads(&layout, plan)?;
    decomposition(&layout, plan)?;
    if let Some(def) = &plan.definition {
        imports(&layout, def)?;
    }
    pins(plan)?;
    profiles(&layout, plan)?;
    if has_blocking(&decomposition::contraction(&plan.entries)) {
        return Err(diag("publication-target-cycle", "plan graph contracts to a target cycle"));
    }
    Ok(())
}

/// Resolve `--from` the same way author does.
#[must_use]
pub fn resolve_from(paths: &ExecutionPaths, from: &Path) -> PathBuf {
    if from.is_absolute() {
        return from.to_path_buf();
    }
    if paths.is_detached() {
        paths.change_root().join(from)
    } else {
        paths.project_root().join(from)
    }
}

fn discovery(layout: &Layout<'_>, plan: &Plan) -> Result<(), Error> {
    let path = layout.discovery_yaml_path();
    match (&plan.discovery_digest, path.is_file()) {
        (None, false) => Ok(()),
        (Some(_), false) => Err(diag(
            "plan-discovery-missing",
            "plan.yaml records discovery-digest but discovery.yaml is absent",
        )),
        (_, true) => {
            let live = Discovery::load(&path)?;
            let digest = live.digest()?;
            if let Some(recorded) = &plan.discovery_digest
                && digest != *recorded
            {
                return Err(diag(
                    "plan-discovery-mismatch",
                    format!(
                        "discovery.yaml digest `{digest}` is not plan.yaml.discovery-digest `{recorded}`"
                    ),
                ));
            }
            if let Some(def) = &plan.definition
                && live.definition != *def
            {
                return Err(diag(
                    "plan-discovery-mismatch",
                    "discovery.yaml definition identity drifted from plan.yaml",
                ));
            }
            if live.sources != plan.sources {
                return Err(diag(
                    "plan-discovery-mismatch",
                    "discovery.yaml sources drifted from plan.yaml",
                ));
            }
            if !targets_match(&live.targets, &plan.targets) {
                return Err(diag(
                    "plan-discovery-mismatch",
                    "discovery.yaml targets drifted from plan.yaml",
                ));
            }
            Ok(())
        }
    }
}

fn targets_match(
    discovery: &std::collections::BTreeMap<String, TargetBinding>,
    plan: &std::collections::BTreeMap<String, TargetBinding>,
) -> bool {
    if discovery.len() != plan.len() {
        return false;
    }
    discovery.iter().all(|(key, row)| {
        plan.get(key).is_some_and(|bound| {
            bound.adapter == row.adapter && bound.locator == row.locator && bound.cid == row.cid
        })
    })
}

fn leads(layout: &Layout<'_>, plan: &Plan) -> Result<(), Error> {
    let Some(recorded) = &plan.leads_digest else {
        return Ok(());
    };
    let catalog = Leads::load(&layout.leads_path())?;
    let live = SnapshotId::from_digest(&catalog.digest_hex()?);
    if live != *recorded {
        return Err(diag(
            "plan-leads-mismatch",
            format!("leads.md digest `{live}` is not plan.yaml.leads-digest `{recorded}`"),
        ));
    }
    if !layout.leads_revision_path(recorded).is_file() {
        return Err(diag(
            "plan-leads-revision-missing",
            format!("retained leads revision `{recorded}` is absent"),
        ));
    }
    Ok(())
}

fn decomposition(layout: &Layout<'_>, plan: &Plan) -> Result<(), Error> {
    let Some(recorded) = &plan.decomposition_digest else {
        return Ok(());
    };
    let tree = Decomposition::load(&layout.decomposition_path())?;
    let live = tree.digest()?;
    if live != *recorded {
        return Err(diag(
            "plan-decomposition-mismatch",
            format!(
                "decomposition.yaml digest `{live}` is not plan.yaml.decomposition-digest `{recorded}`"
            ),
        ));
    }
    if !layout.decomp_revision_path(recorded).is_file() {
        return Err(diag(
            "plan-decomposition-revision-missing",
            format!("retained decomposition revision `{recorded}` is absent"),
        ));
    }
    if plan.leads_digest.as_ref() != Some(&tree.leads_digest) {
        return Err(diag(
            "plan-leads-mismatch",
            "decomposition.yaml.leads-digest does not match plan.yaml.leads-digest",
        ));
    }
    decomposition::matches_plan(&tree, plan)?;
    profiles_vs_tree(plan, &tree)?;
    Ok(())
}

fn profiles_vs_tree(plan: &Plan, tree: &Decomposition) -> Result<(), Error> {
    for (key, row) in &plan.targets {
        let Some(pref) = &row.model_capability_profile else {
            continue;
        };
        let Some(bound) = tree.profiles.get(key) else {
            return Err(diag(
                "plan-profile-mismatch",
                format!("plan target `{key}` profile is absent from decomposition.yaml"),
            ));
        };
        if bound.digest != pref.digest || bound.id != pref.id {
            return Err(diag(
                "plan-profile-mismatch",
                format!("plan target `{key}` profile digest drifted from decomposition.yaml"),
            ));
        }
    }
    Ok(())
}

fn imports(layout: &Layout<'_>, def: &DefinitionIdentity) -> Result<(), Error> {
    let handoff_path = layout.import_handoff_path(&def.handoff_digest);
    if !handoff_path.is_file() {
        return Err(diag(
            "plan-handoff-import-missing",
            format!("imported handoff `{digest}` is absent", digest = def.handoff_digest),
        ));
    }
    let bytes = std::fs::read(&handoff_path).map_err(|source| Error::Filesystem {
        op: "read",
        path: handoff_path.clone(),
        source,
    })?;
    let digest = SnapshotId::from_digest(&sha256_hex(&bytes));
    if digest != def.handoff_digest {
        return Err(diag(
            "plan-handoff-import-mismatch",
            "imported handoff bytes do not match plan.yaml.definition.handoff-digest",
        ));
    }
    let review_path = layout.import_review_path(&def.review.event_digest);
    if !review_path.is_file() {
        return Err(diag(
            "plan-review-import-missing",
            format!(
                "imported review envelope `{digest}` is absent",
                digest = def.review.event_digest
            ),
        ));
    }
    let text = std::fs::read_to_string(&review_path).map_err(|source| Error::Filesystem {
        op: "read",
        path: review_path.clone(),
        source,
    })?;
    let event: Event = serde_json::from_str(text.trim()).map_err(|err| Error::Diag {
        code: "plan-review-import-mismatch",
        detail: format!("imported review envelope is not a journal event: {err}"),
    })?;
    if event.writer != def.review.writer || event.sequence != def.review.sequence {
        return Err(diag(
            "plan-review-import-mismatch",
            "imported review identity drifted from plan.yaml.definition.review",
        ));
    }
    if event.digest()? != def.review.event_digest {
        return Err(diag(
            "plan-review-import-mismatch",
            "imported review bytes do not match plan.yaml.definition.review.event-digest",
        ));
    }
    match &event.kind {
        EventKind::SystemWaveReviewed { handoff_digest, .. }
            if handoff_digest == &def.handoff_digest =>
        {
            Ok(())
        }
        _ => Err(diag(
            "plan-review-import-mismatch",
            "imported review is not system.wave.reviewed for the bound handoff",
        )),
    }
}

fn pins(plan: &Plan) -> Result<(), Error> {
    for (key, source) in &plan.sources {
        source.validate(key)?;
    }
    for entry in &plan.entries {
        plan.target(&entry.target)?;
    }
    Ok(())
}

fn profiles(layout: &Layout<'_>, plan: &Plan) -> Result<(), Error> {
    for entry in &plan.entries {
        let slice_dir = layout.slice_dir(entry.name.as_str());
        let Ok(manifest) = Manifest::load(&slice_dir) else {
            continue;
        };
        let live = refinement::live_profile(plan, entry);
        if manifest.inputs.profile != live {
            return Err(diag(
                "plan-profile-mismatch",
                format!(
                    "slice `{}` refinement profile `{}` is not the plan-row digest `{live}`",
                    entry.name, manifest.inputs.profile
                ),
            ));
        }
    }
    Ok(())
}

fn diag(code: &'static str, detail: impl Into<String>) -> Error {
    Error::Diag {
        code,
        detail: detail.into(),
    }
}

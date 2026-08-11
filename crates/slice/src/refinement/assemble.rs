//! Pure refinement-manifest assembly.
//!
//! IO is limited to digest reads over the baseline and slice trees —
//! no journal, no adapter dispatch, no writes.

use std::collections::BTreeMap;
use std::path::Path;

use artifacts::discovery::Lead;
use error::Error;
use project::adapter::BuildInputDeclaration;
use project::config::Layout;
use project::plan::{Entry, Plan, Projections, contributing_leads, dir_cid};
use project::snapshot::SnapshotId;

use super::{BundleEntry, Dependency, Inputs, Kind, Manifest, Planning, VERSION};
use crate::build::assemble::{
    DESIGN_ARTIFACT, PROPOSAL_ARTIFACT, TASKS_ARTIFACT, resolve_additional, spec_paths,
};

/// Assemble the refinement manifest for `entry`.
///
/// `inventory` is the full `discovery.md` lead set (the contributing
/// closure resolves internally); `target_guidance` is the recorded
/// identity of the guidance text synthesis consumed (supplied by the
/// caller, never recomputed here); `dependencies` are the ordered
/// predecessor `(slice, refinement-digest)` pairs; `declarations` is
/// the bound target's build-inputs list — the bundle covers the same
/// canonical set the target build request assembles.
///
/// # Errors
///
/// - `discovery-lead-unknown` / `plan-projection-source-unbound` from
///   the planning projections.
/// - `slice-refinement-source-unbound` / `slice-refinement-pin-missing`
///   when a bound source has no closed plan pin.
/// - `slice-refinement-input-missing` when a canonical bundle artifact
///   (`proposal.md`, `design.md`, `tasks.md`, or every per-domain
///   spec) is absent; `target-build-input-missing` when a `required`
///   adapter declaration is absent.
/// - Filesystem failures from digest walks.
pub fn assemble(
    layout: Layout<'_>, plan: &Plan, entry: &Entry, inventory: &[Lead],
    target_guidance: SnapshotId, dependencies: Vec<Dependency>,
    declarations: &[BuildInputDeclaration],
) -> Result<Manifest, Error> {
    let contributing = contributing_leads(entry, inventory)?;
    let planning = Projections::compute(plan, entry, &contributing)?;
    let slice_dir = layout.slice_dir(entry.name.as_str());
    Ok(Manifest {
        version: VERSION,
        slice: entry.name.as_str().to_string(),
        inputs: Inputs {
            planning: Planning {
                entry: planning.entry,
                leads: planning.leads,
                decomposition: planning.decomposition,
            },
            profile: super::empty_digest(),
            observations: super::empty_digest(),
            target_guidance,
            baseline_specs: dir_cid(&layout.specs_dir())?,
            sources: source_pins(plan, entry)?,
            dependencies,
        },
        bundle: bundle(&slice_dir, declarations)?,
    })
}

/// Copy per-source `cid` pins from the closed plan source set, exactly
/// as recorded by `plan author`.
fn source_pins(plan: &Plan, entry: &Entry) -> Result<BTreeMap<String, SnapshotId>, Error> {
    let mut sources = BTreeMap::new();
    for binding in &entry.sources {
        let key = binding.source();
        let Some(bound) = plan.sources.get(key) else {
            return Err(Error::Diag {
                code: "slice-refinement-source-unbound",
                detail: format!(
                    "slice `{}` binds source `{key}` which is absent from plan.yaml.sources",
                    entry.name
                ),
            });
        };
        let Some(cid) = bound.cid.clone() else {
            return Err(Error::Diag {
                code: "slice-refinement-pin-missing",
                detail: format!(
                    "source `{key}` has no cid pin; re-run `emery plan author` to close the \
                     source set"
                ),
            });
        };
        sources.insert(key.to_string(), cid);
    }
    Ok(sources)
}

/// The complete output bundle: `proposal.md`, `design.md`, `tasks.md`,
/// the sorted per-domain specs, then the adapter-declared additional
/// inputs in declaration order — the same canonical set
/// [`crate::build::assemble`] resolves into the target build request.
fn bundle(
    slice_dir: &Path, declarations: &[BuildInputDeclaration],
) -> Result<Vec<BundleEntry>, Error> {
    let mut bundle = vec![
        entry(slice_dir, PROPOSAL_ARTIFACT, Kind::Proposal)?,
        entry(slice_dir, DESIGN_ARTIFACT, Kind::Design)?,
        entry(slice_dir, TASKS_ARTIFACT, Kind::Tasks)?,
    ];
    let specs = spec_paths(slice_dir)?;
    if specs.is_empty() {
        return Err(missing("specs/<domain>/spec.md (at least one per-domain spec)"));
    }
    for path in specs {
        bundle.push(entry(slice_dir, &path, Kind::Spec)?);
    }
    for path in resolve_additional(declarations, slice_dir)? {
        bundle.push(entry(slice_dir, &path, Kind::Additional)?);
    }
    Ok(bundle)
}

fn entry(slice_dir: &Path, path: &str, kind: Kind) -> Result<BundleEntry, Error> {
    let digest = super::content_digest(&slice_dir.join(path))?.ok_or_else(|| missing(path))?;
    Ok(BundleEntry {
        path: path.to_string(),
        kind,
        digest,
    })
}

fn missing(path: &str) -> Error {
    Error::Diag {
        code: "slice-refinement-input-missing",
        detail: format!(
            "required refinement input `{path}` is absent from the slice tree; refinement \
             cannot cover an incomplete bundle"
        ),
    }
}

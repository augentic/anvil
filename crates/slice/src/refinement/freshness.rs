//! Freshness projection over a recorded refinement manifest.
//!
//! Recomputes inputs and bundle digests against the live trees;
//! `profile` / `observations` / `target-guidance` stay recorded-only.

use std::collections::BTreeSet;
use std::path::Path;

use artifacts::discovery::Lead;
use diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use error::Error;
use project::config::Layout;
use project::plan::{Entry, Plan, Projections, contributing_leads, dir_cid, source_cid};
use project::snapshot::SnapshotId;

use super::{Kind, Manifest, VERSION, file_digest};
use crate::build::assemble::spec_paths;

/// `emery slice validate` code for an absent refinement manifest.
pub const MISSING_CODE: &str = "slice-refinement-missing";

/// `emery slice validate` code for a stale refinement manifest.
pub const STALE_CODE: &str = "slice-refinement-stale";

/// Result of the freshness projection ([`freshness`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Freshness {
    /// Every recomputable input and bundle digest matches; `digest`
    /// is the refinement digest of the on-disk manifest.
    Fresh {
        /// Content digest of the on-disk manifest bytes.
        digest: SnapshotId,
    },
    /// No manifest exists (pre-refine).
    Missing,
    /// At least one recorded identity no longer matches; one
    /// human-readable reason per mismatch.
    Stale {
        /// One reason per drifted input or bundle artifact.
        reasons: Vec<String>,
    },
}

/// Project the freshness of `entry`'s recorded refinement manifest.
///
/// `inventory` is the full `discovery.md` lead set. Recomputed: the
/// planning projections, `baseline-specs`, per-binding live source
/// digests (binding-set mismatches count as staleness), predecessor
/// refinement digests read from their `refinement.yaml` files (a
/// missing predecessor manifest makes the dependent stale), spec
/// membership, and every recorded bundle file digest. An unparseable
/// manifest is stale, not an error.
///
/// # Errors
///
/// Propagates plan / filesystem failures from live digest walks.
pub fn freshness(
    layout: Layout<'_>, plan: &Plan, entry: &Entry, inventory: &[Lead],
) -> Result<Freshness, Error> {
    let slice_dir = layout.slice_dir(entry.name.as_str());
    let path = Manifest::path(&slice_dir);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            return Ok(Freshness::Missing);
        }
        Err(source) => {
            return Err(Error::Filesystem {
                op: "read",
                path,
                source,
            });
        }
    };
    let manifest: Manifest = match serde_saphyr::from_str(&text) {
        Ok(manifest) => manifest,
        Err(err) => {
            return Ok(Freshness::Stale {
                reasons: vec![format!("refinement.yaml does not parse: {err}")],
            });
        }
    };

    let mut reasons = Vec::new();
    if manifest.version != VERSION {
        reasons.push(format!(
            "manifest version `{}` is not the supported version `{VERSION}`",
            manifest.version
        ));
    }
    if manifest.slice != entry.name.as_str() {
        reasons
            .push(format!("manifest names slice `{}`, expected `{}`", manifest.slice, entry.name));
    }

    planning(plan, entry, inventory, &manifest, &mut reasons);
    baseline(layout, &manifest, &mut reasons)?;
    sources(layout, plan, entry, &manifest, &mut reasons)?;
    dependencies(layout, &manifest, &mut reasons)?;
    bundle(&slice_dir, &manifest, &mut reasons)?;

    if reasons.is_empty() {
        Ok(Freshness::Fresh {
            digest: SnapshotId::from_digest(&diagnostics::digest::sha256_hex(text.as_bytes())),
        })
    } else {
        Ok(Freshness::Stale { reasons })
    }
}

/// Shape one validate-style finding per freshness defect: one
/// [`MISSING_CODE`] finding for an absent manifest, one [`STALE_CODE`]
/// finding per staleness reason, nothing when fresh.
#[must_use]
pub fn findings(name: &str, freshness: &Freshness) -> Vec<Diagnostic> {
    match freshness {
        Freshness::Fresh { .. } => Vec::new(),
        Freshness::Missing => vec![review(
            MISSING_CODE,
            "the slice has a refinement manifest covering its generation inputs",
            format!(
                "slice `{name}` has no refinement.yaml — run `emery plan refine` to generate \
                 and cover its specification bundle"
            ),
        )],
        Freshness::Stale { reasons } => reasons
            .iter()
            .map(|reason| {
                review(
                    STALE_CODE,
                    "the refinement manifest matches its live inputs and bundle",
                    format!(
                        "slice `{name}` refinement is stale: {reason} — re-run `emery plan \
                         refine`"
                    ),
                )
            })
            .collect(),
    }
}

/// Recompute the three planning projections. A failed recompute (a
/// contributing lead or plan-level source binding no longer resolves)
/// is itself staleness: the covered planning input has changed shape.
fn planning(
    plan: &Plan, entry: &Entry, inventory: &[Lead], manifest: &Manifest, reasons: &mut Vec<String>,
) {
    let live = contributing_leads(entry, inventory)
        .and_then(|contributing| Projections::compute(plan, entry, &contributing));
    match live {
        Ok(live) => {
            let recorded = &manifest.inputs.planning;
            for (name, recorded, live) in [
                ("entry", &recorded.entry, &live.entry),
                ("leads", &recorded.leads, &live.leads),
                ("decomposition", &recorded.decomposition, &live.decomposition),
            ] {
                if recorded != live {
                    reasons.push(format!(
                        "planning `{name}` projection `{recorded}` drifted; live digest is \
                         `{live}`"
                    ));
                }
            }
        }
        Err(err) => reasons.push(format!("planning projections no longer compute: {err}")),
    }
}

fn baseline(
    layout: Layout<'_>, manifest: &Manifest, reasons: &mut Vec<String>,
) -> Result<(), Error> {
    let live = dir_cid(&layout.specs_dir())?;
    if live != manifest.inputs.baseline_specs {
        reasons.push(format!(
            "baseline-specs `{}` drifted; live digest is `{live}`",
            manifest.inputs.baseline_specs
        ));
    }
    Ok(())
}

/// Live source digests per binding, mirroring pin-drift semantics:
/// recorded-but-unbound and bound-but-unrecorded keys count as
/// staleness alongside content drift.
fn sources(
    layout: Layout<'_>, plan: &Plan, entry: &Entry, manifest: &Manifest, reasons: &mut Vec<String>,
) -> Result<(), Error> {
    let bound: BTreeSet<&str> =
        entry.sources.iter().map(project::plan::SliceSourceBinding::source).collect();
    for key in manifest.inputs.sources.keys() {
        if !bound.contains(key.as_str()) {
            reasons.push(format!("recorded source `{key}` is no longer bound by the entry"));
        }
    }
    for binding in &entry.sources {
        let key = binding.source();
        let Some(recorded) = manifest.inputs.sources.get(key) else {
            reasons.push(format!("bound source `{key}` has no recorded digest"));
            continue;
        };
        let Some(plan_binding) = plan.sources.get(key) else {
            continue;
        };
        let live = source_cid(key, plan_binding, layout.project_dir())?;
        if live != *recorded {
            reasons.push(format!(
                "source `{key}` digest `{recorded}` drifted; live digest is `{live}`"
            ));
        }
    }
    Ok(())
}

fn dependencies(
    layout: Layout<'_>, manifest: &Manifest, reasons: &mut Vec<String>,
) -> Result<(), Error> {
    for dependency in &manifest.inputs.dependencies {
        match file_digest(&layout.slice_dir(&dependency.slice))? {
            None => reasons
                .push(format!("predecessor `{}` has no refinement manifest", dependency.slice)),
            Some(live) if live != dependency.refinement => reasons.push(format!(
                "predecessor `{}` refinement `{}` drifted; live digest is `{live}`",
                dependency.slice, dependency.refinement
            )),
            Some(_) => {}
        }
    }
    Ok(())
}

/// Recompute spec membership (declaration-free) and every recorded
/// bundle file digest. Adapter-declared additional membership is
/// re-checked only by re-assembly, which needs the target's live
/// declaration set; recorded additional file contents are checked here.
fn bundle(slice_dir: &Path, manifest: &Manifest, reasons: &mut Vec<String>) -> Result<(), Error> {
    let recorded_specs: BTreeSet<&str> = manifest
        .bundle
        .iter()
        .filter(|entry| matches!(entry.kind, Kind::Spec))
        .map(|entry| entry.path.as_str())
        .collect();
    for path in spec_paths(slice_dir)? {
        if !recorded_specs.contains(path.as_str()) {
            reasons.push(format!("spec `{path}` is not covered by the recorded bundle"));
        }
    }
    for entry in &manifest.bundle {
        match super::content_digest(&slice_dir.join(&entry.path))? {
            None => reasons.push(format!("bundle artifact `{}` is missing", entry.path)),
            Some(live) if live != entry.digest => reasons.push(format!(
                "bundle artifact `{}` digest `{}` drifted; live digest is `{live}`",
                entry.path, entry.digest
            )),
            Some(_) => {}
        }
    }
    Ok(())
}

fn review(rule_id: &'static str, title: &str, detail: String) -> Diagnostic {
    Diagnostic::finding(
        rule_id,
        title,
        detail,
        Severity::Suggestion,
        DiagnosticKind::Review,
        DiagnosticSource::Deterministic,
        Artifact::Specs,
        None,
    )
}

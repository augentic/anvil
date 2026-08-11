//! Freshness projection over a recorded refinement manifest.
//!
//! Recomputes inputs and bundle digests against the live trees;
//! `profile` / `observations` / `target-guidance` stay recorded-only.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use artifacts::discovery::Lead;
use diagnostics::{Artifact, Diagnostic, DiagnosticKind, DiagnosticSource, Severity};
use error::Error;

use super::{Kind, Manifest, VERSION, file_digest};
use crate::config::{Layout, ProjectConfig};
use crate::journal::{self, EventKind};
use crate::plan::{Entry, Plan, Projections, SourceBinding, contributing_leads, dir_cid, source_cid};
use crate::snapshot::SnapshotId;

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

/// Per-run cache of the freshness inputs shared by every leaf: the
/// live baseline cid, the newest journaled post-merge baseline
/// (RFC-91 D4), per-source-key live cids, and the declared target
/// binding. Manifest digests are never cached here — the refinement
/// drain rewrites them between checks. One [`Live`] value serves one
/// project root; the shared inputs do not move while a drain runs.
#[derive(Debug, Default)]
pub struct Live {
    baseline: Option<SnapshotId>,
    merged: Option<Option<SnapshotId>>,
    sources: BTreeMap<String, SnapshotId>,
    target: Option<Option<String>>,
}

impl Live {
    /// An empty cache; every input is computed on first use.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Live `.emery/specs/` tree cid, computed once.
    fn baseline(&mut self, layout: Layout<'_>) -> Result<SnapshotId, Error> {
        if self.baseline.is_none() {
            self.baseline = Some(dir_cid(&layout.specs_dir())?);
        }
        Ok(self.baseline.clone().expect("baseline cached above"))
    }

    /// Newest journaled post-merge baseline digest
    /// (`target.merge.wave-committed` `baseline`), read once.
    fn merged(&mut self, layout: Layout<'_>) -> Result<Option<SnapshotId>, Error> {
        if self.merged.is_none() {
            let newest = journal::read_union(layout)?.iter().rev().find_map(|event| {
                match &event.kind {
                    EventKind::TargetMergeWaveCommitted {
                        baseline: Some(baseline),
                        ..
                    } => Some(baseline.clone()),
                    _ => None,
                }
            });
            self.merged = Some(newest);
        }
        Ok(self.merged.clone().expect("merged cached above"))
    }

    /// Live source-tree cid for `key`, computed once per key.
    fn source(
        &mut self, layout: Layout<'_>, key: &str, binding: &SourceBinding,
    ) -> Result<SnapshotId, Error> {
        if let Some(cid) = self.sources.get(key) {
            return Ok(cid.clone());
        }
        let cid = source_cid(key, binding, layout.project_dir())?;
        self.sources.insert(key.to_string(), cid.clone());
        Ok(cid)
    }

    /// Declared target binding from `project.yaml`, loaded once. An
    /// uninitialised root degrades to `None`.
    fn target(&mut self, layout: Layout<'_>) -> Result<Option<String>, Error> {
        if self.target.is_none() {
            let declared = match ProjectConfig::load(layout.project_dir()) {
                Ok(config) => config.adapter,
                Err(Error::NotInitialized) => None,
                Err(err) => return Err(err),
            };
            self.target = Some(declared);
        }
        Ok(self.target.clone().expect("target cached above"))
    }
}

/// One-shot [`freshness_with`]: builds a throwaway [`Live`] cache for
/// callers that check a single leaf (`emery slice validate`).
///
/// # Errors
///
/// Propagates plan / filesystem failures from live digest walks.
pub fn freshness(
    layout: Layout<'_>, plan: &Plan, entry: &Entry, inventory: &[Lead],
) -> Result<Freshness, Error> {
    freshness_with(layout, plan, entry, inventory, &mut Live::new())
}

/// Project the freshness of `entry`'s recorded refinement manifest.
///
/// `inventory` is the full `discovery.md` lead set; `live` memoizes
/// shared inputs across leaves. Recomputes the planning projections,
/// baseline, sources, predecessors (an archived manifest counts,
/// RFC-91 D3), spec membership, and bundle digests; an unparseable
/// manifest is stale, not an error. The baseline also accepts the
/// newest journaled post-merge digest (D4), so plan-local wave
/// commits never stale unbuilt sibling manifests.
///
/// # Errors
///
/// Propagates plan / filesystem failures from live digest walks.
pub fn freshness_with(
    layout: Layout<'_>, plan: &Plan, entry: &Entry, inventory: &[Lead], live: &mut Live,
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

    planning(plan, entry, inventory, &manifest, live.target(layout)?.as_deref(), &mut reasons);
    baseline(layout, &manifest, live, &mut reasons)?;
    sources(layout, plan, entry, &manifest, live, &mut reasons)?;
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

/// Refinement digest of `slice`'s manifest, falling back to the
/// newest archive entry when the live slice tree has none: merge and
/// `plan drop` move the whole tree — `refinement.yaml` included — to
/// `.emery/archive/<stamp>-<slice>/`, and an accepted predecessor
/// satisfies "predecessor refined" a fortiori (RFC-91 D3). `None`
/// only when neither a live nor an archived manifest exists.
///
/// # Errors
///
/// Filesystem read failures other than absence.
pub fn predecessor_digest(layout: Layout<'_>, slice: &str) -> Result<Option<SnapshotId>, Error> {
    if let Some(digest) = file_digest(&layout.slice_dir(slice))? {
        return Ok(Some(digest));
    }
    match latest_archive(&layout.archive_dir(), slice) {
        Some(dir) => file_digest(&dir),
        None => Ok(None),
    }
}

/// The newest `<YYYY-MM-DD>-<slice>` folder under the archive root,
/// by the leading stamp's lexicographic order. Best-effort read-only:
/// an unreadable archive root yields `None`.
#[must_use]
pub fn latest_archive(archive_dir: &Path, slice: &str) -> Option<PathBuf> {
    const DATE_PREFIX_LEN: usize = "0000-00-00-".len();
    let mut best: Option<String> = None;
    for entry in std::fs::read_dir(archive_dir).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let dated_match = name.len() == DATE_PREFIX_LEN + slice.len()
            && name.ends_with(slice)
            && name.as_bytes().get(DATE_PREFIX_LEN - 1) == Some(&b'-');
        if dated_match && entry.path().is_dir() && best.as_deref() < Some(name.as_str()) {
            best = Some(name);
        }
    }
    best.map(|name| archive_dir.join(name))
}

/// Recompute the three planning projections. A failed recompute (a
/// contributing lead or plan-level source binding no longer resolves)
/// is itself staleness: the covered planning input has changed shape.
fn planning(
    plan: &Plan, entry: &Entry, inventory: &[Lead], manifest: &Manifest, target: Option<&str>,
    reasons: &mut Vec<String>,
) {
    let live = contributing_leads(entry, inventory)
        .and_then(|contributing| Projections::compute(plan, entry, &contributing, target));
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

/// The recorded `baseline-specs` pin against the live tree, with the
/// RFC-91 D4 plan-local carve-out: a live tree matching the newest
/// journaled post-merge baseline is accepted drift, not staleness.
fn baseline(
    layout: Layout<'_>, manifest: &Manifest, live: &mut Live, reasons: &mut Vec<String>,
) -> Result<(), Error> {
    let current = live.baseline(layout)?;
    if current == manifest.inputs.baseline_specs {
        return Ok(());
    }
    if live.merged(layout)?.is_some_and(|newest| newest == current) {
        return Ok(());
    }
    reasons.push(format!(
        "baseline-specs `{}` drifted; live digest is `{current}`",
        manifest.inputs.baseline_specs
    ));
    Ok(())
}

/// Live source digests per binding: recorded-but-unbound and
/// bound-but-unrecorded keys count as staleness alongside content
/// drift.
fn sources(
    layout: Layout<'_>, plan: &Plan, entry: &Entry, manifest: &Manifest, live: &mut Live,
    reasons: &mut Vec<String>,
) -> Result<(), Error> {
    let bound: BTreeSet<&str> =
        entry.sources.iter().map(crate::plan::SliceSourceBinding::source).collect();
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
        let current = live.source(layout, key, plan_binding)?;
        if current != *recorded {
            reasons.push(format!(
                "source `{key}` digest `{recorded}` drifted; live digest is `{current}`"
            ));
        }
    }
    Ok(())
}

/// Recorded predecessor pins against each predecessor's current
/// manifest digest — the archived manifest when the live tree is gone
/// ([`predecessor_digest`]).
fn dependencies(
    layout: Layout<'_>, manifest: &Manifest, reasons: &mut Vec<String>,
) -> Result<(), Error> {
    for dependency in &manifest.inputs.dependencies {
        match predecessor_digest(layout, &dependency.slice)? {
            None => reasons
                .push(format!("predecessor `{}` has no refinement manifest", dependency.slice)),
            Some(current) if current != dependency.refinement => reasons.push(format!(
                "predecessor `{}` refinement `{}` drifted; live digest is `{current}`",
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
    for path in crate::slice::spec_paths(slice_dir)? {
        if !recorded_specs.contains(path.as_str()) {
            reasons.push(format!("spec `{path}` is not covered by the recorded bundle"));
        }
    }
    for entry in &manifest.bundle {
        match super::content_digest(&slice_dir.join(&entry.path))? {
            None => reasons.push(format!("bundle artifact `{}` is missing", entry.path)),
            Some(current) if current != entry.digest => reasons.push(format!(
                "bundle artifact `{}` digest `{}` drifted; live digest is `{current}`",
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

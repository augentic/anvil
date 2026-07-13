//! Generic artefact-class slice consumed by the merge engine. Callers
//! supply an ordered `&[ArtifactClass]`; the engine dispatches on
//! [`MergeStrategy`] and never matches on [`ArtifactClass::name`].

use std::path::{Path, PathBuf};

use crate::config::Layout;

/// One mutable artefact class that participates in a slice's merge.
///
/// Each class carries the staged location (under the slice / change
/// directory), the baseline location (relative to the project root),
/// and the [`MergeStrategy`] used to promote staged content into the
/// baseline.
///
/// The [`ArtifactClass::name`] field is for diagnostic output only.
/// The merge engine MUST NOT branch on it; promotion behaviour is
/// driven by [`ArtifactClass::strategy`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArtifactClass {
    /// Identifier from the adapter or call site (e.g. `"specs"` or
    /// `"contracts"` for the omnia-default synthesiser). Used purely
    /// for diagnostics and the merge-summary string. The engine never
    /// branches on this field.
    pub name: String,
    /// Where the slice stages this class. Absolute path — typically a
    /// child of the change directory but the engine treats it as an
    /// opaque location.
    pub staged_dir: PathBuf,
    /// Where the baseline lives. Absolute path — typically rooted at
    /// the project root but, again, opaque to the engine.
    pub baseline_dir: PathBuf,
    /// How staged content is promoted into the baseline.
    pub strategy: MergeStrategy,
}

/// Strategy for promoting an [`ArtifactClass`]'s staged content into
/// its baseline.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MergeStrategy {
    /// 3-way merge of textual content. The engine scans
    /// `<staged_dir>/<name>/spec.md` files (one per spec name) and
    /// merges each delta into the corresponding baseline file at
    /// `<baseline_dir>/<name>/spec.md`. Today's `omnia` "specs"
    /// behaviour. Also pulls in a top-level `composition.yaml` from
    /// the change directory when present (omnia / vectis convention).
    ThreeWayMerge,
    /// Whole-file replacement. The engine walks `<staged_dir>`
    /// recursively and copies each file to the corresponding path
    /// under `<baseline_dir>`, overwriting any existing baseline file.
    /// Today's `omnia` "contracts" behaviour.
    OpaqueReplace,
}

/// Default omnia [`ArtifactClass`] set: `specs` (3-way merge) and
/// `contracts` (opaque replace).
///
/// Single source of truth shared by the native merge verbs, the
/// synthesize baseline resolution, and the guest orchestrators; future
/// adapter manifests should drive this through `specify-adapter`.
#[must_use]
pub fn artifact_classes(project_root: &Path, slice_dir: &Path) -> Vec<ArtifactClass> {
    vec![
        ArtifactClass {
            name: "specs".to_string(),
            staged_dir: slice_dir.join("specs"),
            baseline_dir: Layout::new(project_root).specify_dir().join("specs"),
            strategy: MergeStrategy::ThreeWayMerge,
        },
        ArtifactClass {
            name: "contracts".to_string(),
            staged_dir: slice_dir.join("contracts"),
            baseline_dir: project_root.join("contracts"),
            strategy: MergeStrategy::OpaqueReplace,
        },
    ]
}

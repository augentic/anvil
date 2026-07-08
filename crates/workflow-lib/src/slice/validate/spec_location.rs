//! Spec file-location gate. Flags a slice that carries a root-level
//! `spec.md` but no canonical `specs/<domain>/spec.md` files.

use std::path::Path;

use specify_diagnostics::{Artifact, Diagnostic};

use super::collect_spec_files;

/// Spec file-location gate. Emits a `specs.file-location`
/// finding when the slice has no spec files under the canonical
/// `specs/<domain>/spec.md` layout but does have a root-level
/// `spec.md`. This fires first among the pre-adapter gates so the
/// operator sees the structural cause before downstream drift noise.
pub(super) fn collect_spec_file_location_findings(slice_dir: &Path) -> Vec<Diagnostic> {
    let specs_dir = slice_dir.join("specs");
    let has_canonical_specs =
        specs_dir.is_dir() && collect_spec_files(&specs_dir).is_ok_and(|files| !files.is_empty());
    if has_canonical_specs {
        return Vec::new();
    }
    let root_spec = slice_dir.join("spec.md");
    if !root_spec.is_file() {
        return Vec::new();
    }
    vec![Diagnostic::violation(
        "specs.file-location",
        "Spec files live under specs/<domain>/spec.md, not at the slice root",
        "No spec files found under `specs/`. Found `spec.md` at the slice root — \
         move it to `specs/<domain>/spec.md` (one file per `proposal.md ## Domains` entry). \
         The Specify workflow requires spec files under `specs/` for every target.",
        Artifact::Specs,
        None,
    )]
}

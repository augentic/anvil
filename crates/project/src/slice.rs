//! The slice data model shared across the stack: `metadata.yaml`,
//! projected lifecycle labels, the phase-outcome record, and the
//! requirement-body digest; the slice loop lives in the `slice` crate.

pub mod lifecycle;
pub mod metadata;
pub mod outcome;
pub mod requirement;

pub use lifecycle::{LifecycleStatus, has_spec_artifacts};
pub use metadata::{
    Outcome, SLICES_DIR_NAME, SliceMetadata, SpecKind, TouchedSpec, slice_not_found,
};
pub use outcome::Kind as OutcomeKind;
pub use requirement::RequirementBody;

/// Refinement-manifest filename inside a slice directory (RFC-91 D4).
/// The manifest DTO and freshness kernel live in [`crate::refinement`];
/// this const keeps the projections here reading the same path.
pub const REFINEMENT_FILE: &str = "refinement.yaml";

/// Whether `slice_dir` holds a refinement manifest. Presence-only:
/// staleness needs [`crate::refinement::freshness()`]'s full recompute.
#[must_use]
pub fn refinement_present(slice_dir: &std::path::Path) -> bool {
    slice_dir.join(REFINEMENT_FILE).is_file()
}

/// Sorted `specs/<domain>/spec.md` paths (slice-tree relative) for each
/// domain directory under `<slice_tree>/specs/` carrying a `spec.md`.
///
/// Returns an empty vector when `specs/` is missing — the build-request
/// schema (`specs` `minItems: 1`) catches an empty list downstream.
/// Shared by build-request assembly and the refinement bundle so both
/// cover the same canonical set.
///
/// # Errors
///
/// Propagates a `specs/` directory read failure.
pub fn spec_paths(slice_tree: &std::path::Path) -> Result<Vec<String>, error::Error> {
    let specs_dir = slice_tree.join("specs");
    if !specs_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths: Vec<String> = Vec::new();
    for entry in crate::fs::dir_entries(&specs_dir)? {
        let domain_dir = entry.path();
        if !domain_dir.is_dir() || !domain_dir.join("spec.md").is_file() {
            continue;
        }
        if let Some(domain) = domain_dir.file_name().and_then(std::ffi::OsStr::to_str) {
            paths.push(format!("specs/{domain}/spec.md"));
        }
    }
    paths.sort();
    Ok(paths)
}

//! Pure build-request assembly.
//!
//! [`build_request`] is IO-free apart from existence checks against
//! the slice tree — it never writes a journal, request file, or report.

use std::path::Path;

use error::{Error, Result};
use project::adapter::BuildInputDeclaration;
use project::seam::wire::{
    BUILD_VERSION, BuildArtifacts, BuildInputs, BuildRequest, DeferredRequirement,
};

pub const PROPOSAL_ARTIFACT: &str = "proposal.md";
pub const DESIGN_ARTIFACT: &str = "design.md";
pub const TASKS_ARTIFACT: &str = "tasks.md";

/// Assemble a [`BuildRequest`] for `slice` from already-resolved
/// inputs.
///
/// `manifest_inputs` is the bound target's declared build-inputs list;
/// `slice_tree` is the tree all artifact paths resolve against;
/// `project_dir` is the working tree the target builds into.
///
/// The `specs[]` are the per-domain `spec.md` files found under the
/// slice tree (sorted); `additional[]` resolves in declaration order;
/// `deferred` is the caller-projected live deferred set (RFC-86a D4).
///
/// # Errors
///
/// - [`Error::Validation`] keyed on `target-build-input-missing` (exit
///   code 2) when a `required` declaration names a path absent from the
///   slice tree.
/// - [`Error::Filesystem`] when the slice tree's `specs/` directory
///   exists but cannot be read.
pub fn build_request(
    slice: &str, manifest_inputs: &[BuildInputDeclaration], slice_tree: &Path, project_dir: &Path,
    deferred: Vec<DeferredRequirement>,
) -> Result<BuildRequest> {
    let specs = spec_paths(slice_tree)?;
    let additional = resolve_additional(manifest_inputs, slice_tree)?;
    Ok(BuildRequest {
        version: BUILD_VERSION,
        slice: slice.to_string(),
        project_dir: project_dir.to_path_buf(),
        inputs: BuildInputs {
            root: slice_tree.to_path_buf(),
            artifacts: BuildArtifacts {
                proposal: PROPOSAL_ARTIFACT.to_string(),
                design: DESIGN_ARTIFACT.to_string(),
                tasks: TASKS_ARTIFACT.to_string(),
                specs,
                additional,
            },
        },
        deferred,
    })
}

pub use project::slice::spec_paths;

/// Resolve the manifest input declarations against the slice tree.
///
/// Present declarations contribute their path (declaration order);
/// absent optional declarations are skipped; an absent `required`
/// declaration aborts. Shared with the refinement bundle so both
/// cover the same canonical set.
pub fn resolve_additional(
    manifest_inputs: &[BuildInputDeclaration], slice_tree: &Path,
) -> Result<Vec<String>> {
    let mut additional: Vec<String> = Vec::new();
    for decl in manifest_inputs {
        if slice_tree.join(&decl.path).exists() {
            additional.push(decl.path.clone());
        } else if decl.required {
            return Err(Error::validation_failed(
                "target-build-input-missing",
                "required adapter-declared build input is present in the slice tree",
                format!("required input `{}` is absent from the slice tree", decl.path),
            ));
        }
    }
    Ok(additional)
}

//! The shared source-input preparation kernel.
//!
//! Local tree or inline value → immutable snapshot → read-only
//! workspace → the WIT `source-input`; the wire never carries a locator.

use std::path::{Path, PathBuf};

use error::Error;

use super::Workspaces;
use crate::plan::SourceBinding;
use crate::snapshot::SnapshotId;

/// The prepared input a source operation reads — mirrors the WIT
/// `source-input` variant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceInput {
    /// Deployment-local root of a prepared read-only tree (RFC-87).
    Workspace(String),
    /// Raw content of a single-value binding, interpolated rather
    /// than lent.
    Inline(String),
}

/// What one source binding materializes from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Material {
    /// A deployment-local tree: a directory, or a single file
    /// snapshotted as a one-file tree.
    Tree(PathBuf),
    /// An inline single-value binding.
    Value(String),
}

/// One prepared source input plus the provenance the caller may
/// persist or journal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedInput {
    /// The wire input for the dispatch.
    pub input: SourceInput,
    /// Content identity of the prepared tree — `Store::snapshot` for
    /// trees, the one-file `content` encoding (`value_cid`) inline.
    pub cid: SnapshotId,
    /// Read-only workspace to discard after the dispatch; `None` for
    /// inline inputs, which prepare no workspace.
    pub workspace: Option<String>,
}

/// Resolve one `plan.yaml` source binding into its material. A
/// relative `path` joins `root`; `value` is inline.
///
/// # Errors
///
/// `source-input-unbound` when the binding carries neither `path` nor
/// `value`.
pub fn binding_material(
    key: &str, binding: &SourceBinding, root: &Path,
) -> Result<Material, Error> {
    if let Some(path) = binding.path.as_deref() {
        let path = Path::new(path);
        let resolved = if path.is_absolute() { path.to_path_buf() } else { root.join(path) };
        return Ok(Material::Tree(resolved));
    }
    if let Some(value) = binding.value.as_deref() {
        return Ok(Material::Value(value.to_string()));
    }
    Err(Error::Diag {
        code: "source-input-unbound",
        detail: format!("source `{key}` has neither `path` nor `value`; nothing to prepare"),
    })
}

/// Prepare one source input: snapshot the tree and materialize a
/// read-only workspace, or pass an inline value straight through.
///
/// The caller dispatches the operation, then discards
/// [`PreparedInput::workspace`] via [`discard`] — snapshot objects
/// survive by digest but are never delivery GC roots.
///
/// # Errors
///
/// `source-input-prepare` wrapping snapshot / materialization
/// failures from the workspace kernel.
pub async fn prepare(
    workspaces: &impl Workspaces, material: Material,
) -> Result<PreparedInput, Error> {
    match material {
        Material::Tree(path) => {
            let path = path.display().to_string();
            let cid = workspaces.snapshot(path).await.map_err(|err| failure(&err))?;
            let prepared =
                workspaces.prepare(cid.clone(), false).await.map_err(|err| failure(&err))?;
            Ok(PreparedInput {
                input: SourceInput::Workspace(prepared.root),
                cid,
                workspace: Some(prepared.id),
            })
        }
        Material::Value(value) => Ok(PreparedInput {
            cid: crate::plan::value_cid(&value),
            input: SourceInput::Inline(value),
            workspace: None,
        }),
    }
}

/// Best-effort discard of a prepared read-only workspace. Failures
/// are swallowed: the age-based workspace GC is the backstop, and a
/// discard failure must never mask the dispatch result.
pub async fn discard(workspaces: &impl Workspaces, prepared: PreparedInput) {
    if let Some(id) = prepared.workspace {
        let _dropped = workspaces.discard(id).await;
    }
}

fn failure(err: &super::Error) -> Error {
    Error::Diag {
        code: "source-input-prepare",
        detail: format!("preparing the source input failed: {err}"),
    }
}

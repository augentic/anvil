//! Shared source-operation prep seam.
//!
//! [`prepare`] factors the environment prep that `source preview`
//! (workflow-free) and the `source survey` / `source extract` runners
//! (workflow-integrated) all build on:
//!
//! 1. adapter resolution via [`SourceAdapter::resolve`];
//! 2. brief-directory resolution (the `briefs-dir` = adapter root
//!    joined `briefs/`);
//! 3. the four-root sandbox preopen [`SandboxLayout`] — `$SOURCE_DIR`,
//!    `$CAPABILITY_DIR`, `$SCRATCH_DIR`, `$PROJECT_DIR` — as data,
//!    with the per-operation scratch path keyed by operation; and
//! 4. `evidence/` scaffolding for the output target.
//!
//! It produces *prep*, not behaviour: the agent handoff, the journal
//! events, validate-before-visible, and the `discovery.md` merge /
//! Evidence persist are what the workflow-integrated layer adds on
//! top — none of it lives here.

use std::path::{Path, PathBuf};

use specify_error::{Error, Result};
use specify_workflow::adapter::{AdapterRef, SourceAdapter};

use crate::runtime::commands::BRIEFS_DIR;

/// `evidence/` subdirectory scaffolded under the output target.
const EVIDENCE_DIR: &str = "evidence";

/// Scratch-tree segment for the slice-less `survey` operation.
/// Kept in sync with the `survey` operation's wire
/// spelling by `survey_segment_matches_operation_wire_name`.
const SURVEY_SCRATCH_SEGMENT: &str = "survey";

/// Sandbox env var names — the unprefixed keys the WASI host binds
/// (mirroring `crates/tool/src/host.rs`'s `PROJECT_DIR` /
/// `CAPABILITY_DIR`).
const SOURCE_DIR_VAR: &str = "SOURCE_DIR";
const CAPABILITY_DIR_VAR: &str = "CAPABILITY_DIR";
const SCRATCH_DIR_VAR: &str = "SCRATCH_DIR";
const PROJECT_DIR_VAR: &str = "PROJECT_DIR";

/// Which source operation a prep targets, carrying the data needed to
/// key its `$SCRATCH_DIR`.
///
/// `survey` runs at plan time with no slice and keys scratch under the
/// literal `survey/` segment; `extract` runs at slice time and keys
/// scratch under `<slice>/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceOp {
    /// Plan-time lead discovery — scratch under `…/scratch/survey/`.
    Survey,
    /// Slice-time evidence extraction — scratch under
    /// `…/scratch/<slice>/`. Constructed by the `extract`
    /// runner ([`crate::runtime::commands::source::extract`]).
    Extract {
        /// Slice name keying the scratch directory.
        slice: String,
    },
}

impl SourceOp {
    /// Scratch-lane segment under `scratch/<adapter>/` for this
    /// operation: the literal `survey` for the slice-less survey op,
    /// the slice name for slice-time extract.
    fn scratch_segment(&self) -> &str {
        match self {
            Self::Survey => SURVEY_SCRATCH_SEGMENT,
            Self::Extract { slice } => slice,
        }
    }
}

/// Access mode for a source-adapter sandbox preopen root. The data side
/// of the WASI `DirPerms` / `FilePerms` the runner will mount each
/// root with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreopenAccess {
    /// Mounted read-only.
    ReadOnly,
    /// Mounted write-only.
    WriteOnly,
    /// Not mounted — not visible to the adapter operation.
    None,
}

/// One source-adapter sandbox preopen root: the env var the adapter
/// reads, its access mode, and the host path (absent when the root is
/// not mounted — `$PROJECT_DIR` always, `$SOURCE_DIR` for value-bound
/// sources).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preopen {
    /// Sandbox env var name (`SOURCE_DIR`, `CAPABILITY_DIR`,
    /// `SCRATCH_DIR`, `PROJECT_DIR`).
    pub var: &'static str,
    /// Declared access mode.
    pub access: PreopenAccess,
    /// Host path to mount, or `None` when the root is not visible.
    pub path: Option<PathBuf>,
}

/// The four-root source-adapter sandbox preopen layout.
///
/// Data only: this computes the roots and their modes. The agent
/// handoff and the journal events are the workflow-integrated layer's
/// job.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxLayout {
    /// `$SOURCE_DIR` — read-only bound source path; absent
    /// (`access: None`, `path: None`) for value-bound sources.
    pub source: Preopen,
    /// `$CAPABILITY_DIR` — read-only resolved adapter manifest cache
    /// (the adapter root the manifest loaded from).
    pub capability: Preopen,
    /// `$SCRATCH_DIR` — write-only per-operation scratch lane under
    /// `.specify/scratch/`.
    pub scratch: Preopen,
    /// `$PROJECT_DIR` — not visible to the adapter operation.
    pub project: Preopen,
}

impl SandboxLayout {
    /// Build the four roots from a bound source path (`None` for
    /// value-bound sources), the resolved adapter root, and the
    /// per-operation scratch path.
    fn new(source: Option<&Path>, capability: &Path, scratch: PathBuf) -> Self {
        Self {
            source: Preopen {
                var: SOURCE_DIR_VAR,
                access: source.map_or(PreopenAccess::None, |_| PreopenAccess::ReadOnly),
                path: source.map(Path::to_path_buf),
            },
            capability: Preopen {
                var: CAPABILITY_DIR_VAR,
                access: PreopenAccess::ReadOnly,
                path: Some(capability.to_path_buf()),
            },
            scratch: Preopen {
                var: SCRATCH_DIR_VAR,
                access: PreopenAccess::WriteOnly,
                path: Some(scratch),
            },
            project: Preopen {
                var: PROJECT_DIR_VAR,
                access: PreopenAccess::None,
                path: None,
            },
        }
    }
}

/// Inputs to [`prepare`].
pub struct PrepRequest<'a> {
    /// Kebab-case source-adapter name.
    pub adapter: &'a str,
    /// Optional semver pin from the source bind (RFC-47 D2). `None`
    /// resolves the single installed identity.
    pub version: Option<semver::Version>,
    /// Project directory containing `adapters/` and `.specify/`.
    pub project_dir: &'a Path,
    /// Operation the prep targets — drives the `$SCRATCH_DIR` keying.
    pub op: SourceOp,
    /// Bound source path → `$SOURCE_DIR`. `None` for value-bound
    /// sources (e.g. `intent`), where `$SOURCE_DIR` is absent.
    pub source: Option<&'a Path>,
    /// Canonical lead selection echoed back for the handoff envelope.
    /// One spelling across the family: `preview --lead <id>…` and
    /// `extract <lead>` both fold into this slice.
    pub leads: &'a [String],
    /// Output target whose `evidence/` subtree is scaffolded — the
    /// `--out` dir for preview, `.specify/slices/<slice>/` for
    /// extract. `None` for survey (no Evidence output).
    pub evidence_root: Option<&'a Path>,
}

/// Result of [`prepare`] — the shared prep all source-operation verbs
/// build on.
pub struct SourcePrep {
    /// Resolved source-adapter manifest (name, version, briefs,
    /// execution mode).
    pub manifest: SourceAdapter,
    /// Resolved adapter root (= `$CAPABILITY_DIR`).
    pub adapter_dir: PathBuf,
    /// `<adapter-root>/briefs/` — the resolve-envelope `briefs-dir`.
    /// Surfaced in the `survey` handoff envelope; preview surfaces brief
    /// paths directly from [`Self::adapter_dir`].
    pub briefs_dir: PathBuf,
    /// Four-root sandbox preopen layout (data only). The `survey` runner
    /// reads its `scratch` / `source` roots; the `extract` runner
    /// wires the full WASI preopen set.
    pub layout: SandboxLayout,
    /// Scaffolded `evidence/` directory, or `None` when the operation
    /// writes no Evidence (survey).
    pub evidence_dir: Option<PathBuf>,
    /// Canonical lead selection echoed back for the handoff envelope.
    pub leads: Vec<String>,
}

/// Resolve the adapter, compute the brief directory and four-root
/// sandbox layout, and scaffold the `evidence/` output target.
///
/// # Errors
///
/// Propagates [`SourceAdapter::resolve`] failures (`adapter-not-found`,
/// `adapter-schema-violation`, …) and I/O failures from the `evidence/`
/// directory create.
pub fn prepare(request: &PrepRequest<'_>) -> Result<SourcePrep> {
    let adapter_ref = AdapterRef {
        name: request.adapter.to_string(),
        version: request.version.clone(),
    };
    let resolved = SourceAdapter::resolve(&adapter_ref, request.project_dir)?;
    let adapter_dir = resolved.location.path().clone();
    let briefs_dir = adapter_dir.join(BRIEFS_DIR);

    let scratch = scratch_dir(request.project_dir, &resolved.manifest.name, &request.op);
    let layout = SandboxLayout::new(request.source, &adapter_dir, scratch);

    let evidence_dir = match request.evidence_root {
        Some(root) => {
            let dir = root.join(EVIDENCE_DIR);
            std::fs::create_dir_all(&dir).map_err(Error::Io)?;
            Some(dir)
        }
        None => None,
    };

    Ok(SourcePrep {
        manifest: resolved.manifest,
        adapter_dir,
        briefs_dir,
        layout,
        evidence_dir,
        leads: request.leads.to_vec(),
    })
}

/// Resolve a `plan.yaml.sources.<key>.path` binding against
/// `plan_root`: absolute paths pass through, relative paths join onto
/// the directory `plan.yaml` lives in (the project root, or the
/// initiating workspace under a `--plan-dir` override) — relative
/// bindings are authored against the plan's home. Shared by the
/// `survey` and `extract` runners so the `$SOURCE_DIR` host path is
/// computed in one place.
#[must_use]
pub fn resolve_source_path(plan_root: &Path, raw: &str) -> PathBuf {
    let candidate = Path::new(raw);
    if candidate.is_absolute() { candidate.to_path_buf() } else { plan_root.join(candidate) }
}

/// `.specify/scratch/<adapter>/<segment>/`, where `<segment>` is
/// `survey` for the slice-less survey op or the slice name for
/// extract. Rooted under the transient working-state tree
/// (`.specify/scratch/`), disjoint from the out-of-tree memoization
/// tree (the per-project OS cache), so a scratch write never shares a
/// namespace with a cache artifact.
fn scratch_dir(project_dir: &Path, adapter: &str, op: &SourceOp) -> PathBuf {
    specify_workflow::adapter::scratch_dir(project_dir, adapter, op.scratch_segment())
}

#[cfg(test)]
mod tests {
    use specify_workflow::adapter::SourceOperation;

    use super::*;

    #[test]
    fn survey_segment_matches_wire_name() {
        assert_eq!(SURVEY_SCRATCH_SEGMENT, SourceOperation::Survey.to_string());
    }

    #[test]
    fn scratch_keys_under_survey_segment() {
        let scratch = scratch_dir(Path::new("/proj"), "documentation", &SourceOp::Survey);
        assert_eq!(scratch, Path::new("/proj/.specify/scratch/documentation/survey"));
    }

    #[test]
    fn extract_scratch_keys_under_slice_segment() {
        let op = SourceOp::Extract {
            slice: "identity-password-reset".to_string(),
        };
        let scratch = scratch_dir(Path::new("/proj"), "typescript", &op);
        assert_eq!(scratch, Path::new("/proj/.specify/scratch/typescript/identity-password-reset"));
    }

    #[test]
    fn path_bound_mounts_four_roots() {
        let source = PathBuf::from("/repo/legacy");
        let capability = PathBuf::from("/proj/adapters/sources/typescript");
        let scratch = PathBuf::from("/proj/.specify/scratch/typescript/s");
        let layout = SandboxLayout::new(Some(&source), &capability, scratch.clone());

        assert_eq!(layout.source.var, "SOURCE_DIR");
        assert_eq!(layout.source.access, PreopenAccess::ReadOnly);
        assert_eq!(layout.source.path, Some(source));

        assert_eq!(layout.capability.var, "CAPABILITY_DIR");
        assert_eq!(layout.capability.access, PreopenAccess::ReadOnly);
        assert_eq!(layout.capability.path, Some(capability));

        assert_eq!(layout.scratch.var, "SCRATCH_DIR");
        assert_eq!(layout.scratch.access, PreopenAccess::WriteOnly);
        assert_eq!(layout.scratch.path, Some(scratch));

        assert_eq!(layout.project.var, "PROJECT_DIR");
        assert_eq!(layout.project.access, PreopenAccess::None);
        assert_eq!(layout.project.path, None);
    }

    #[test]
    fn value_bound_source_dir_absent() {
        let capability = PathBuf::from("/proj/adapters/sources/intent");
        let scratch = PathBuf::from("/proj/.specify/scratch/intent/survey");
        let layout = SandboxLayout::new(None, &capability, scratch);

        assert_eq!(layout.source.access, PreopenAccess::None);
        assert_eq!(layout.source.path, None, "value-bound source has no $SOURCE_DIR");
        // The other three roots are unaffected by the absent source.
        assert_eq!(layout.capability.access, PreopenAccess::ReadOnly);
        assert_eq!(layout.scratch.access, PreopenAccess::WriteOnly);
        assert_eq!(layout.project.access, PreopenAccess::None);
    }
}

//! On-disk Evidence reading and distillation.
//!
//! Owns the typed `evidence/*.yaml` readers shared by the validation
//! sweep and the provenance projection, and distils the per-source
//! `authority` map, the `(source, id) → kind` claim anchor index, and
//! the per-source inputs envelope rows for the guest refine
//! orchestrator.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use artifacts::evidence::{AuthorityClass, ClaimKind, Document};
use error::{Error, Result};
use project::config::Layout;
use project::plan::Entry;

use crate::synthesis::wire::SourceInput;

/// Sorted paths to `.yaml`/`.yml` files under `<slice_dir>/evidence/`.
///
/// The walk is non-recursive: only direct children of `evidence/` whose
/// extension is `yaml` or `yml` are considered. Returns an empty
/// vector when `evidence/` is missing or not a directory.
///
/// # Errors
///
/// - [`Error::Filesystem`] if `evidence/` exists but cannot be read.
pub fn evidence_yaml_paths(slice_dir: &Path) -> Result<Vec<PathBuf>> {
    let evidence_dir = slice_dir.join("evidence");
    if !evidence_dir.is_dir() {
        return Ok(Vec::new());
    }

    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in project::fs::dir_entries(&evidence_dir)? {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let ext = path.extension().and_then(OsStr::to_str).unwrap_or("");
        if ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

/// One parsed `evidence/<source>.yaml` from a single read.
///
/// Read, typed-parsed, and validated once by [`read_evidence_dir`] and
/// handed back so the downstream slice gates (catalog-drift and
/// model-drift) reuse the typed document instead of re-reading and
/// re-parsing the file from disk.
#[derive(Debug)]
pub struct EvidenceDoc {
    /// Source key derived from the file stem.
    pub source: String,
    /// The typed Evidence document.
    pub document: Document,
}

/// Read every `*.yaml` file under `<slice_dir>/evidence/` into a typed
/// [`Document`], running each document's deterministic validation.
///
/// The evidence subdirectory is optional — returning an empty `Vec`
/// when it is absent matches the workflow §Extraction reliability rule
/// that an empty `claims: []` (or no Evidence at all before extract
/// runs) is valid.
///
/// All findings are aggregated and returned in a single
/// [`Error::Validation`] keyed on `evidence-schema` so the caller sees
/// every malformed file in one pass.
///
/// # Errors
///
/// - [`Error::Filesystem`] if `evidence/` exists but cannot be read.
/// - [`Error::Validation`] if any Evidence file fails the YAML parse,
///   the typed parse, or the deterministic document checks.
pub fn read_evidence_dir(slice_dir: &Path) -> Result<Vec<EvidenceDoc>> {
    let mut docs: Vec<EvidenceDoc> = Vec::new();
    let mut findings: Vec<String> = Vec::new();
    for path in evidence_yaml_paths(slice_dir)? {
        let source = path.file_stem().and_then(OsStr::to_str).unwrap_or_default().to_string();
        let parsed = project::fs::read_text(&path)
            .and_then(|raw| serde_saphyr::from_str::<Document>(&raw).map_err(Error::from));
        match parsed {
            Ok(document) => match document.validate() {
                Ok(()) => docs.push(EvidenceDoc { source, document }),
                Err(err) => findings.push(format!("{}: {err}", path.display())),
            },
            Err(err) => findings.push(format!("{}: {err}", path.display())),
        }
    }

    if findings.is_empty() {
        Ok(docs)
    } else {
        Err(Error::Validation {
            code: "evidence-schema".into(),
            detail: findings.join("; "),
        })
    }
}

/// The two kernel projection inputs distilled from on-disk Evidence:
/// the per-source document-level `authority` map and the
/// `(source, id) → kind` claim anchor index.
pub type KernelEvidence = (BTreeMap<String, AuthorityClass>, BTreeMap<(String, String), ClaimKind>);

/// Read each bound source's `evidence/<source>.yaml` into a
/// [`SourceInput`] for the agent inputs envelope, carrying the
/// project-relative `evidence-path` the agent reads the claims from.
///
/// # Errors
///
/// Propagates Evidence read and parse failures.
pub fn read_source_inputs(layout: Layout<'_>, entry: &Entry) -> Result<Vec<SourceInput>> {
    let slice_dir = layout.slice_dir(&entry.name);
    entry
        .sources
        .iter()
        .map(|binding| {
            let source = binding.source();
            let path = evidence_path(&slice_dir, source);
            SourceInput::from_file(source, &path, wire_path(layout, &path))
        })
        .collect()
}

/// Project-relative, `/`-joined form of `path` — the lent-tree path
/// the synthesis inputs envelope hands the agent.
fn wire_path(layout: Layout<'_>, path: &Path) -> String {
    path.strip_prefix(layout.project_dir())
        .unwrap_or(path)
        .components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Distil the per-source document-level `authority` map and the
/// `(source, id) → kind` anchor index the kernel projects against, from
/// each bound source's on-disk Evidence.
///
/// # Errors
///
/// Propagates Evidence read and parse failures.
pub fn read_evidence_index(slice_dir: &Path, entry: &Entry) -> Result<KernelEvidence> {
    let mut authority: BTreeMap<String, AuthorityClass> = BTreeMap::new();
    let mut claims: BTreeMap<(String, String), ClaimKind> = BTreeMap::new();
    for binding in &entry.sources {
        let source = binding.source().to_string();
        let path = evidence_path(slice_dir, &source);
        let raw = project::fs::read_text(&path)?;
        let document: Document = serde_saphyr::from_str(&raw)?;
        authority.insert(source.clone(), document.authority);
        for claim in document.claims {
            if let Some(id) = claim.id {
                claims.insert((source.clone(), id), claim.kind);
            }
        }
    }
    Ok((authority, claims))
}

/// `<slice_dir>/evidence/<source>.yaml`.
#[must_use]
pub fn evidence_path(slice_dir: &Path, source: &str) -> PathBuf {
    slice_dir.join("evidence").join(format!("{source}.yaml"))
}

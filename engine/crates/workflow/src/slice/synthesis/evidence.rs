//! On-disk Evidence distillation for the synthesis kernel.
//!
//! Distils the per-source `authority` map, the `(source, id) → kind`
//! claim anchor index, and the per-source inputs envelope rows. Shared
//! by the native `slice synthesize` handler and the guest refine
//! orchestrator.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde_json::Value as JsonValue;
use specify_error::{Error, Result};
use specify_model::evidence::{AuthorityClass, ClaimKind};

use crate::change::Entry;
use crate::slice::synthesis::wire::SynthesisSourceInput;

/// The two kernel projection inputs distilled from on-disk Evidence:
/// the per-source document-level `authority` map and the
/// `(source, id) → kind` claim anchor index.
pub type KernelEvidence = (BTreeMap<String, AuthorityClass>, BTreeMap<(String, String), ClaimKind>);

/// Read each bound source's `evidence/<source>.yaml` into a
/// [`SynthesisSourceInput`] for the agent inputs envelope.
///
/// # Errors
///
/// Propagates Evidence read and parse failures.
pub fn read_source_inputs(slice_dir: &Path, entry: &Entry) -> Result<Vec<SynthesisSourceInput>> {
    entry
        .sources
        .iter()
        .map(|binding| {
            let source = binding.source();
            let path = evidence_path(slice_dir, source);
            SynthesisSourceInput::from_evidence_file(source, &path)
        })
        .collect()
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
        let raw = std::fs::read_to_string(&path).map_err(|err| Error::Filesystem {
            op: "read",
            path: path.clone(),
            source: err,
        })?;
        let doc: JsonValue = serde_saphyr::from_str(&raw)?;
        if let Some(class) = doc.get("authority").and_then(JsonValue::as_str).and_then(parse_enum) {
            authority.insert(source.clone(), class);
        }
        let Some(doc_claims) = doc.get("claims").and_then(JsonValue::as_array) else {
            continue;
        };
        for claim in doc_claims {
            let (Some(id), Some(kind)) = (
                claim.get("id").and_then(JsonValue::as_str),
                claim.get("kind").and_then(JsonValue::as_str).and_then(parse_enum),
            ) else {
                continue;
            };
            claims.insert((source.clone(), id.to_string()), kind);
        }
    }
    Ok((authority, claims))
}

/// `<slice_dir>/evidence/<source>.yaml`.
#[must_use]
pub fn evidence_path(slice_dir: &Path, source: &str) -> PathBuf {
    slice_dir.join("evidence").join(format!("{source}.yaml"))
}

/// Parse one kebab-case enum value out of a JSON string, mirroring the
/// `EvidenceIndex::read` pattern in `slice/model.rs`.
fn parse_enum<T: serde::de::DeserializeOwned>(value: &str) -> Option<T> {
    serde_json::from_value(JsonValue::String(value.to_string())).ok()
}

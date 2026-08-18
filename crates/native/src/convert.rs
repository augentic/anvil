//! SDK-seam to engine-seam DTO conversion — the one native copy of
//! the mapping the wasm guest shim applies at the WIT boundary.

use adapter::seam as aseam;
use project::adapter::metadata::Metadata;

/// Project SDK source metadata onto the engine resolver metadata.
#[must_use]
pub fn source_metadata(record: aseam::SourceMetadata) -> Metadata {
    Metadata {
        emery_floor: record.emery_floor,
        inputs: Vec::new(),
        platforms: None,
        writable_artifacts: Vec::new(),
    }
}

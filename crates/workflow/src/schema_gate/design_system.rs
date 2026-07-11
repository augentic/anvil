//! Design-system gates: the operator-curated component catalog and
//! parts inputs.

use std::path::Path;

use error::Result;
use schema::{COMPONENTS_JSON_SCHEMA, PARTS_JSON_SCHEMA};

use super::support::validate_labelled_yaml;

/// Validate raw `components.yaml` content against the embedded
/// `schemas/design-system/components.schema.json`.
///
/// `source_path` labels error messages with the originating file.
///
/// # Errors
///
/// Returns [`error::Error::Validation`] when YAML parsing or schema
/// validation fails.
pub fn validate_components_yaml(content: &str, source_path: &Path) -> Result<()> {
    validate_labelled_yaml(
        content,
        source_path,
        COMPONENTS_JSON_SCHEMA,
        "catalog-schema",
        "components.yaml conforms to schemas/design-system/components.schema.json",
    )
}

/// Validate raw `parts.yaml` content against the embedded
/// `schemas/design-system/parts.schema.json`.
///
/// `source_path` labels error messages with the originating file.
/// Backs [`crate::design_system::Parts::load`] — the schema is the only
/// gate on the operator-authored parts input ("beyond
/// schema conformance there are no coherence gates").
///
/// # Errors
///
/// Returns [`error::Error::Validation`] when YAML parsing or schema
/// validation fails.
pub fn validate_parts_yaml(content: &str, source_path: &Path) -> Result<()> {
    validate_labelled_yaml(
        content,
        source_path,
        PARTS_JSON_SCHEMA,
        "parts-schema",
        "parts.yaml conforms to schemas/design-system/parts.schema.json",
    )
}

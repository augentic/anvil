//! Design-system gate for the operator-curated component catalog.

use std::path::Path;

use error::Result;
use schema::COMPONENTS_JSON_SCHEMA;

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

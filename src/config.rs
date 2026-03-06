use std::path::Path;

use anyhow::Result;

use crate::pipeline::RepoGroup;
use crate::registry::Registry;
use crate::spec_engine::SpecEngine;

/// Generate a merged config for a target repo by delegating to the spec engine.
pub fn generate(
    engine: &dyn SpecEngine, base_config: &Path, registry: &Registry, group: &RepoGroup,
) -> Result<String> {
    engine.generate_config(base_config, registry, group)
}

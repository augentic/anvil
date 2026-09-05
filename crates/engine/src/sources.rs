//! Per-run source bindings: the transport-neutral binding DTO and the
//! rules every binding obeys before anything loads.
//!
//! The binding list is an input, never engine state: nothing here is
//! persisted, and the engine never writes a binding list anywhere.

use std::path::Path;

use omnia_guest::plugins::Digest;
use omnia_guest::{Error, bad_request};
use serde::{Deserialize, Serialize};

use crate::preopen::preopen_path;
use crate::resolve::AdapterSelector;

/// A source binding for one run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceBinding {
    /// Stable binding key.
    pub key: String,
    /// The adapter selector value.
    pub adapter: String,
    /// What the adapter extracts.
    pub content: BindingContent,
    /// Optional sha256 content pin for a loader-loaded adapter,
    /// verified host-side before validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<Digest>,
    /// Optional registry endpoint override for a package adapter;
    /// `None` selects the acquirer's default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

/// Workspace or inline source content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BindingContent {
    /// Project-relative read-only root; `.` binds the project.
    Workspace(String),
    /// Inline description text; no filesystem view.
    Description(String),
}

/// Checks a run's binding list before anything loads.
///
/// # Errors
///
/// Returns a `BadRequest` for an empty list (`specify-source-required`),
/// a repeated key, a malformed selector, a `digest` on a bare name the
/// loader never acquires, a `registry` on a selector the registry never
/// serves, or a workspace root outside the project preopen.
pub fn validate(bindings: &[SourceBinding]) -> Result<(), Error> {
    if bindings.is_empty() {
        return Err(source_required());
    }
    for (index, binding) in bindings.iter().enumerate() {
        if bindings[..index].iter().any(|earlier| earlier.key == binding.key) {
            return Err(bad_request!(
                "each source binds once: source `{}` is bound twice",
                binding.key
            ));
        }
        let selector = AdapterSelector::parse(&binding.adapter)?;
        selector.name()?;
        registry_allowed(binding, &selector)?;
        digest_allowed(binding, &selector)?;
        if let BindingContent::Workspace(relative) = &binding.content {
            preopen_path(Path::new(relative))?;
        }
    }
    Ok(())
}

// The endpoint override only steers registry acquisition, so it rides
// only a package-shaped selector.
fn registry_allowed(binding: &SourceBinding, selector: &AdapterSelector) -> Result<(), Error> {
    if binding.registry.is_some() && !matches!(selector, AdapterSelector::Package { .. }) {
        return Err(bad_request!(
            "source `{}` sets `registry` on an adapter the registry never serves; the override \
             only applies to registry package references (`<namespace>:<name>@<version>`)",
            binding.key
        ));
    }
    Ok(())
}

// The pin binds exact component bytes, so it rides only a selector
// the loader acquires — a local component path or a registry package.
fn digest_allowed(binding: &SourceBinding, selector: &AdapterSelector) -> Result<(), Error> {
    if binding.digest.is_some() && matches!(selector, AdapterSelector::Bare { .. }) {
        return Err(bad_request!(
            "source `{}` sets `digest` on a bare adapter name the loader never acquires; pin a \
             local component path or an exact registry package reference instead",
            binding.key
        ));
    }
    Ok(())
}

fn source_required() -> Error {
    Error::BadRequest {
        code: "specify-source-required".into(),
        description: "a specification run requires at least one source binding".into(),
    }
}

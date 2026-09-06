//! Per-run source bindings: the transport-neutral binding DTO and the
//! rules every binding obeys before anything loads. The list is an
//! input, never engine state.

use std::path::{Path, PathBuf};

use emery_source::Source;
use emery_source::types::{SourceContent, SourceInput, SourceWorkspace};
use omnia_guest::plugins::Digest;
use omnia_guest::{Error, Plugins, bad_request};
use serde::{Deserialize, Serialize};

use crate::preopen::preopen_path;
use crate::resolve::{AdapterSelector, Resolved, Resolver};

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
        return Err(Error::BadRequest {
            code: "specify-source-required".into(),
            description: "a specification run requires at least one source binding".into(),
        });
    }

    for (index, binding) in bindings.iter().enumerate() {
        if bindings[..index].iter().any(|earlier| earlier.key == binding.key) {
            return Err(bad_request!(
                "each source binds once: source `{}` is bound twice",
                binding.key
            ));
        }
        binding.validate()?;
    }

    Ok(())
}

/// A source binding for one run.
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl SourceBinding {
    // Resolves this binding's adapter under its pin and registry override.
    pub(crate) async fn resolve<P: Source + Plugins>(
        &self, resolver: &Resolver<'_, P>,
    ) -> Result<Resolved, Error> {
        let selector = AdapterSelector::parse(&self.adapter)?;
        resolver.resolve(&selector, self.digest.as_ref(), self.registry.as_deref()).await
    }

    // Maps this binding to the adapter `extract` input.
    pub(crate) fn input(&self) -> Result<SourceInput, Error> {
        let content = match &self.content {
            BindingContent::Workspace(relative) => {
                // `.` spans the project preopen, including `.emery/`, until
                // guest capability profiles can exclude the output home.
                let relative = preopen_path(Path::new(relative))?;
                let root = if relative == Path::new(".") {
                    PathBuf::from(".")
                } else {
                    Path::new(".").join(&relative)
                };
                SourceContent::Workspace(SourceWorkspace {
                    id: self.key.clone(),
                    root: root.display().to_string(),
                })
            }
            BindingContent::Description(text) => SourceContent::Value(text.clone()),
        };

        Ok(SourceInput {
            key: self.key.clone(),
            content,
        })
    }

    fn validate(&self) -> Result<(), Error> {
        let selector = AdapterSelector::parse(&self.adapter)?;
        selector.name()?;
        self.registry_allowed(&selector)?;
        self.digest_allowed(&selector)?;
        if let BindingContent::Workspace(relative) = &self.content {
            preopen_path(Path::new(relative))?;
        }
        Ok(())
    }

    // The endpoint override only steers registry acquisition, so it rides
    // only a package-shaped selector.
    fn registry_allowed(&self, selector: &AdapterSelector) -> Result<(), Error> {
        if self.registry.is_some() && !matches!(selector, AdapterSelector::Package { .. }) {
            return Err(bad_request!(
                "source `{}` sets `registry` on an adapter the registry never serves; the override \
                 only applies to registry package references (`<namespace>:<name>@<version>`)",
                self.key
            ));
        }
        Ok(())
    }

    // The pin binds exact component bytes, so it rides only a selector
    // the loader acquires — a local component path or a registry package.
    fn digest_allowed(&self, selector: &AdapterSelector) -> Result<(), Error> {
        if self.digest.is_some() && matches!(selector, AdapterSelector::Bare { .. }) {
            return Err(bad_request!(
                "source `{}` sets `digest` on a bare adapter name the loader never acquires; pin a \
                 local component path or an exact registry package reference instead",
                self.key
            ));
        }
        Ok(())
    }
}

/// Workspace or inline source content.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BindingContent {
    /// Project-relative read-only root; `.` binds the project.
    Workspace(String),
    /// Inline description text; no filesystem view.
    Description(String),
}

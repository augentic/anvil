//! Source bindings
//!
//! A [`SourceBinding`] names one source a `specify` run should extract from:
//! the adapter to use, the key the specification will cite it by, and either
//! a workspace to read or an inline value. [`validate`] checks a run's whole
//! list before any adapter is loaded.
//!
//! The list is per-run input, never stored, so the same shape serves the
//! command line, a config file, and any other transport. Checking it up
//! front means a malformed binding is refused before a single adapter runs.

use std::collections::BTreeSet;
use std::path::Path;

use emery_source::Source;
use emery_source::types::{SourceContent, SourceInput, SourceWorkspace};
use omnia_guest::plugins::Digest;
use omnia_guest::{Error, Plugins, bad_request};
use serde::{Deserialize, Serialize};

use crate::plugin::{AdapterRef, Loaded, Loader};
use crate::{is_kebab, preopen_path};

/// Checks a run's binding list before anything loads.
///
/// # Errors
///
/// Returns a `BadRequest` for an empty list (`specify-source-required`),
/// a malformed or repeated key, a `digest` on a bare name the loader
/// never acquires, a `registry` on a selector the registry never
/// serves, or a workspace root outside the project preopen.
pub fn validate(bindings: &[SourceBinding]) -> Result<(), Error> {
    if bindings.is_empty() {
        return Err(Error::BadRequest {
            code: "specify-source-required".into(),
            description: "no source bindings".into(),
        });
    }

    let mut keys = BTreeSet::new();
    for binding in bindings {
        let key = binding.key.as_str();
        if !is_kebab(key) {
            return Err(bad_request!("source `{key}` is not a kebab-case key"));
        }
        if !keys.insert(key) {
            return Err(bad_request!("source `{key}` is bound twice"));
        }
        binding.validate()?;
    }

    Ok(())
}

/// A source binding for one run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceBinding {
    /// Stable kebab-case binding key.
    pub key: String,
    /// The adapter selector.
    pub adapter: AdapterRef,
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
    // Loads this binding's adapter under its pin and registry override.
    pub(crate) async fn load<P: Source + Plugins>(
        &self, loader: &Loader<'_, P>,
    ) -> Result<Loaded, Error> {
        loader.load(&self.adapter, self.digest.as_ref(), self.registry.as_deref()).await
    }

    // Maps this binding to the adapter `extract` input.
    pub(crate) fn input(&self) -> Result<SourceInput, Error> {
        Ok(SourceInput {
            key: self.key.clone(),
            content: self.content.source(&self.key)?,
        })
    }

    // `registry` steers only registry acquisition and `digest` pins only
    // loader-acquired bytes; the root rule is the one `input` applies.
    fn validate(&self) -> Result<(), Error> {
        let key = &self.key;
        if self.registry.is_some() && !matches!(self.adapter, AdapterRef::Package { .. }) {
            return Err(bad_request!(
                "source `{key}`: `registry` requires a package adapter \
                 (`<namespace>:<name>@<version>`)"
            ));
        }
        if self.digest.is_some() && matches!(self.adapter, AdapterRef::Bare(_)) {
            return Err(bad_request!(
                "source `{key}`: `digest` requires a `.wasm` path or package adapter, not a bare \
                 name"
            ));
        }
        self.input()?;

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

impl BindingContent {
    // The one place a content variant meets the adapter's input.
    fn source(&self, key: &str) -> Result<SourceContent, Error> {
        Ok(match self {
            // `.` spans the project preopen, including `.emery/`, until
            // guest capability profiles can exclude the revision store.
            Self::Workspace(relative) => {
                let relative = preopen_path(Path::new(relative))?;
                let root = if relative == Path::new(".") {
                    relative
                } else {
                    Path::new(".").join(relative)
                };
                SourceContent::Workspace(SourceWorkspace {
                    id: key.to_owned(),
                    root: root.display().to_string(),
                })
            }
            Self::Description(text) => SourceContent::Value(text.clone()),
        })
    }
}

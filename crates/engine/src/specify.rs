//! The `specify` operation
//!
//! Emery's central operation: given a list of source bindings, extract each
//! source's claims, derive the requirement rows under authority precedence,
//! synthesise `spec.md` and `design.md`, and commit the pair as one new
//! revision.
//!
//! A [`SourceBinding`] names one source to extract from: the adapter to use,
//! the key the specification will cite it by, and either a workspace to read
//! or an inline value. The list is per-run input, never stored, so the same
//! shape serves the command line, a config file, and any other transport,
//! and it is checked whole before a single adapter loads.
//!
//! The result reports what was committed — the revision id, the counts, the
//! resolved adapter digests an operator can pin, and the diff against the
//! superseded revision — so a caller can see what changed without reading
//! the documents.

mod draft;
mod extract;
mod judgment;
mod provenance;
mod render;
mod synthesise;

use std::collections::BTreeSet;
use std::path::Path;

use emery_source::Source;
use emery_source::types::{SourceContent, SourceInput, SourceWorkspace};
use omnia_guest::api::Context;
use omnia_guest::plugins::Digest;
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore, bad_request};
use serde::{Deserialize, Serialize};

use self::extract::extract;
use self::synthesise::synthesise;
use crate::plugin::{AdapterRef, Loaded, Loader};
use crate::store::Store;
pub use crate::store::{Changes, Diff};
use crate::{is_kebab, preopen_path};

/// Run one `specify` over the context's provider.
///
/// # Errors
///
/// Returns `BadRequest` for a binding the rules refuse or a claim the gate
/// rejects, and passes through the extract, synthesis, and store failures.
pub async fn specify<P: Model + Source + StateStore + BlobStore + Plugins>(
    input: Specify, context: Context<P>,
) -> Result<SpecifyBody, Error> {
    let Specify { bindings } = input;
    validate(&bindings)?;

    let provider = context.provider();
    let sets = extract(provider, &bindings).await?;
    let rows = provenance::derive(provider, &sets).await?;
    let revision = synthesise(provider, &sets, &rows).await?;
    let committed = Store::new(provider).commit(&revision).await?;

    let digests = sets
        .iter()
        .filter_map(|set| {
            Some(SourceDigest {
                source: set.key.clone(),
                digest: set.digest.clone()?,
            })
        })
        .collect();

    Ok(SpecifyBody {
        revision: committed.id,
        requirements: rows.len(),
        sources: sets.len(),
        diff: committed.diff,
        digests,
    })
}

/// Generate a specification revision from source bindings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Specify {
    /// The run's source bindings, in extraction order.
    pub bindings: Vec<SourceBinding>,
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
    async fn load<P: Source + Plugins>(&self, loader: &Loader<'_, P>) -> Result<Loaded, Error> {
        loader.load(&self.adapter, self.digest.as_ref(), self.registry.as_deref()).await
    }

    // Maps this binding to the adapter `extract` input.
    fn input(&self) -> Result<SourceInput, Error> {
        Ok(SourceInput {
            key: self.key.clone(),
            content: self.content.source(&self.key)?,
        })
    }

    // `registry` steers only registry acquisition and `digest` pins only
    // loader-acquired bytes; the root rule is the one `input` applies, so
    // the whole list is refused before any adapter loads.
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

        self.input().map(drop)
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

/// Successful specification result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyBody {
    /// Committed revision id.
    pub revision: String,
    /// Number of committed requirements.
    pub requirements: usize,
    /// Number of extracted sources.
    pub sources: usize,
    /// Diff from the predecessor; absent on the first run and when the
    /// superseded revision was unreadable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
    /// Resolved digests of loader-loaded adapters; commit one as its
    /// binding's `digest` pin to make the load reproducible.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub digests: Vec<SourceDigest>,
}

/// One loader-resolved source digest reported by `emery specify`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceDigest {
    /// The binding key.
    pub source: String,
    /// The resolved `sha256:<hex>` content digest.
    pub digest: Digest,
}

// Refuses an empty list (`specify-source-required`), a malformed or repeated
// key, a `digest` on a bare name the loader never acquires, a `registry` on
// a selector the registry never serves, or a root outside the preopen.
fn validate(bindings: &[SourceBinding]) -> Result<(), Error> {
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

//! The `specify` operation
//!
//! Emery's central operation: given a list of sources, extract each
//! source's claims, derive the requirement rows under authority precedence,
//! synthesise `spec.md` and `design.md`, and commit the pair as one new
//! revision.
//!
//! A [`SourceConfig`] names one source to extract from: the adapter to use,
//! the key the specification will cite it by, and either a workspace to read
//! or an inline value. The list is per-run input, never stored, so the same
//! shape serves the command line, a config file, and any other transport,
//! and it is checked whole before a single adapter loads.
//!
//! The result reports what was committed — the revision id, the counts, the
//! resolved adapter digests an operator can pin, and the diff against the
//! superseded revision — so a caller can see what changed without reading
//! the documents.

mod answer;
mod extract;
mod provenance;
mod render;
mod synthesise;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use emery_source::Source;
use emery_source::claims::is_kebab;
pub use emery_source::types::SourceContent;
use emery_source::types::SourceInput;
use omnia_guest::api::Context;
use omnia_guest::plugins::Digest;
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore, bad_request};
use serde::{Deserialize, Serialize};

use self::extract::extract;
use self::synthesise::synthesise;
use crate::plugin::{AdapterRef, Loaded, Loader};
use crate::preopen_path;
use crate::store::Store;
pub use crate::store::{Changes, Diff};

/// Run one `specify` over the context's provider.
///
/// # Errors
///
/// Returns `BadRequest` for a source the rules refuse or a claim the gate
/// rejects, and passes through the extract, synthesis, and store failures.
pub async fn specify<P: Model + Source + StateStore + BlobStore + Plugins>(
    input: Specify, context: Context<P>,
) -> Result<SpecifyBody, Error> {
    let Specify { sources } = input;
    validate(&sources)?;

    let provider = context.provider();
    let (extracted, digests) = extract(provider, &sources).await?;
    let rows = provenance::derive(provider, &extracted).await?;
    let revision = synthesise(provider, &extracted, &rows).await?;
    let committed = Store::new(provider).commit(&revision).await?;

    Ok(SpecifyBody {
        revision: committed.id,
        requirements: rows.len(),
        sources: extracted.len(),
        diff: committed.diff,
        digests,
    })
}

/// Generate a specification revision from sources.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Specify {
    /// The run's source configurations, in extraction order.
    pub sources: Vec<SourceConfig>,
}

/// A source for one run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SourceConfig {
    /// Stable kebab-case source key.
    pub key: String,
    /// The adapter selector.
    pub adapter: AdapterRef,
    /// What the adapter extracts: a project-relative read-only root
    /// (`.` binds the project) or an inline value.
    pub content: SourceContent,
    /// Optional sha256 content pin for a loader-loaded adapter,
    /// verified host-side before validation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub digest: Option<Digest>,
    /// Optional registry endpoint override for a package adapter;
    /// `None` selects the acquirer's default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry: Option<String>,
}

impl SourceConfig {
    // Loads this source's adapter under its pin and registry override.
    async fn load<P: Source + Plugins>(&self, loader: &Loader<'_, P>) -> Result<Loaded, Error> {
        loader.load(&self.adapter, self.digest.as_ref(), self.registry.as_deref()).await
    }

    // Maps this source to the adapter `extract` input; the one place an
    // operator root meets the guest preopen.
    fn input(&self) -> Result<SourceInput, Error> {
        let content = match &self.content {
            // `.` spans the project preopen, including `.emery/`, until
            // guest capability profiles can exclude the revision store.
            SourceContent::Workspace(relative) => {
                let relative = preopen_path(Path::new(relative))?;
                let root = if relative == Path::new(".") {
                    relative
                } else {
                    Path::new(".").join(relative)
                };
                SourceContent::Workspace(root.display().to_string())
            }
            SourceContent::Value(text) => SourceContent::Value(text.clone()),
        };
        Ok(SourceInput {
            key: self.key.clone(),
            content,
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
    /// The resolved `sha256:<hex>` digest of each loader-loaded adapter,
    /// by source key; commit one as its source's `digest` pin to make the
    /// load reproducible.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub digests: BTreeMap<String, Digest>,
}

// Refuses an empty list (`specify-source-required`), a malformed or repeated
// key, a `digest` on a bare name the loader never acquires, a `registry` on
// a selector the registry never serves, or a root outside the preopen.
fn validate(sources: &[SourceConfig]) -> Result<(), Error> {
    if sources.is_empty() {
        return Err(Error::BadRequest {
            code: "specify-source-required".into(),
            description: "no sources".into(),
        });
    }

    let mut keys = BTreeSet::new();
    for source in sources {
        let key = source.key.as_str();
        if !is_kebab(key) {
            return Err(bad_request!("source `{key}` is not a kebab-case key"));
        }
        if !keys.insert(key) {
            return Err(bad_request!("source `{key}` appears twice"));
        }
        source.validate()?;
    }

    Ok(())
}

// Joins the synthesis prose at `paths` into one system prompt.
fn system(paths: &[&str]) -> String {
    paths.iter().map(|path| crate::prose::body(path)).collect::<Vec<_>>().join("\n\n---\n\n")
}

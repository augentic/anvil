//! The `emery specify` operation.

use std::fmt;

use emery_source::Source;
use omnia_guest::api::{Context, Handler};
use omnia_guest::plugins::Digest;
use omnia_guest::{BlobStore, Error, Model, Plugins, StateStore};
use serde::{Deserialize, Serialize};

use crate::extract::extract_all;
use crate::home::{Diff, Home, SpecSet};
use crate::synthesise::{reconcile, synthesise};

/// Input for `emery specify` — the run's source bindings.
///
/// The input doubles as the verb's clap surface; field docs are its
/// `--help` text.
#[derive(Debug, Default, Clone, Serialize, Deserialize, clap::Args)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyInput {
    /// Workspace-backed source adapters or local component paths.
    #[serde(default)]
    pub adapters: Vec<String>,
    /// Bind an inline source as `<adapter>=<text>`; repeatable.
    #[serde(default)]
    #[arg(long = "description", short = 'd')]
    pub descriptions: Vec<String>,
    /// Operator-owned config; the omitted value selects emery.toml.
    #[serde(default)]
    #[arg(long, short = 'c', num_args = 0..=1, default_missing_value = crate::sources::CONFIG_FILE)]
    pub config: Option<String>,
}

/// Successful `emery specify` result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyBody {
    /// Committed generation id.
    pub generation: String,
    /// Number of committed requirements.
    pub requirements: usize,
    /// Number of extracted sources.
    pub sources: usize,
    /// Diff from the predecessor; absent on the first run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diff: Option<Diff>,
    /// Resolved content digests of loader-loaded adapters (local
    /// components and registry packages) — commit one as its
    /// binding's `digest` pin to make the load reproducible
    /// (trust-on-first-use).
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

impl fmt::Display for SpecifyBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "committed generation {}", self.generation)?;
        writeln!(f, "  requirements: {}", self.requirements)?;
        writeln!(f, "  sources: {}", self.sources)?;
        for entry in &self.digests {
            writeln!(f, "  digest {}: {}", entry.source, entry.digest)?;
        }
        if let Some(diff) = &self.diff {
            if diff.is_empty() {
                writeln!(f, "  diff vs {}: none (byte-stable)", diff.from)?;
            } else {
                writeln!(f, "  diff vs {}: {}", diff.from, diff.artifacts.join(", "))?;
                for subject in &diff.added {
                    writeln!(f, "    + {subject}")?;
                }
                for subject in &diff.removed {
                    writeln!(f, "    - {subject}")?;
                }
                for subject in &diff.changed {
                    writeln!(f, "    ~ {subject}")?;
                }
            }
        }
        Ok(())
    }
}

impl<P: Model + Source + StateStore + BlobStore + Plugins> Handler<P> for SpecifyInput {
    type Error = Error;
    type Output = SpecifyBody;

    async fn handle(self, context: Context<'_, P>) -> Result<Self::Output, Self::Error> {
        let Self {
            adapters,
            descriptions,
            config,
        } = self;
        let bindings = crate::sources::bindings(&adapters, &descriptions, config.as_deref())?;

        let sets = extract_all(context.provider, &bindings).await?;
        let rows = reconcile(&sets);
        let documents = synthesise(context.provider, &sets, &rows).await?;

        let set = SpecSet {
            spec: documents.spec,
            design: documents.design,
        };
        let home = Home::new(context.provider);
        // One observation feeds both the CAS expected value and the
        // re-mine diff, computed in memory and emitted only here.
        let observed = home.observe().await;
        let committed = home.commit(&set, &observed).await?;
        let diff =
            observed.into_outgoing().map(|(from, previous)| Diff::between(from, &previous, &set));
        let digests = sets
            .iter()
            .filter_map(|source| {
                source.digest.clone().map(|digest| SourceDigest {
                    source: source.key.clone(),
                    digest,
                })
            })
            .collect();
        Ok(SpecifyBody {
            generation: committed.id,
            requirements: rows.len(),
            sources: sets.len(),
            diff,
            digests,
        })
    }
}

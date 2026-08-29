//! The `emery specify` operation.

use std::io::Write;

use emery_adapter::Source;
use omnia_guest::api::{Context, Handler};
use omnia_guest::{BlobStore, Error, Model, StateStore};
use serde::{Deserialize, Serialize};

use crate::extract::extract_all;
use crate::handler::Render;
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
    #[arg(long = "value")]
    pub values: Vec<String>,
    /// Operator-owned binding list; defaults to sources.toml.
    #[serde(default)]
    #[arg(long, num_args = 0..=1, default_missing_value = "sources.toml")]
    pub sources: Option<String>,
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
}

impl Render for SpecifyBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "committed generation {}", self.generation)?;
        writeln!(w, "  requirements: {}", self.requirements)?;
        writeln!(w, "  sources: {}", self.sources)?;
        if let Some(diff) = &self.diff {
            if diff.is_empty() {
                writeln!(w, "  diff vs {}: none (byte-stable)", diff.from)?;
            } else {
                writeln!(w, "  diff vs {}: {}", diff.from, diff.artifacts.join(", "))?;
                for subject in &diff.added {
                    writeln!(w, "    + {subject}")?;
                }
                for subject in &diff.removed {
                    writeln!(w, "    - {subject}")?;
                }
                for subject in &diff.changed {
                    writeln!(w, "    ~ {subject}")?;
                }
            }
        }
        Ok(())
    }
}

impl<P: Model + Source + StateStore + BlobStore> Handler<P> for SpecifyInput {
    type Error = Error;
    type Output = SpecifyBody;

    async fn handle(self, context: Context<'_, P>) -> Result<Self::Output, Self::Error> {
        let Self {
            adapters,
            values,
            sources,
        } = self;
        let bindings = crate::sources::bindings(&adapters, &values, sources.as_deref())?;

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
        Ok(SpecifyBody {
            generation: committed.id,
            requirements: rows.len(),
            sources: sets.len(),
            diff,
        })
    }
}

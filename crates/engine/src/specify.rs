//! The `emery specify` operation.

use std::io::Write;

use emery_adapter::Source;
use omnia_guest::api::Provider;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use omnia_guest::{BlobStore, Model, StateStore};
use serde::{Deserialize, Serialize};

use crate::extract::extract_all;
use crate::handler::{ExecutionPaths, Render};
use crate::home::{Diff, Home, SpecSet};
use crate::synthesise::{reconcile, synthesise};

/// Input for `emery specify` — the run's source bindings.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecifyInput {
    /// Source adapters bound as workspace-backed sources.
    #[serde(default)]
    pub adapters: Vec<String>,
    /// Value-backed source bindings, each `<adapter>=<text>`.
    #[serde(default)]
    pub values: Vec<String>,
    /// Path of an operator-owned `sources.toml` carrying the bindings.
    #[serde(default)]
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

/// The `specify` operation route.
#[derive(Clone, Copy, Debug)]
pub struct Specify;

impl<P: Provider + Model + Source + StateStore + BlobStore> Operation<P> for Specify {
    type Error = crate::handler::Error;
    type Input = SpecifyInput;
    type Output = SpecifyBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let SpecifyInput {
            adapters,
            values,
            sources,
        } = input;
        let paths = ExecutionPaths::deployed();
        let bindings = crate::sources::bindings(&adapters, &values, sources.as_deref())?;

        let sets = extract_all(context.provider, &bindings, &paths).await?;
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

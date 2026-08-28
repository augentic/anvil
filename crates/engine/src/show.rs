//! The `emery show` operation: a verifiable, non-authoritative
//! projection of the current generation to stdout.

use std::io::Write;

use omnia_guest::api::{Context, Handler};
use omnia_guest::{BlobStore, Error, StateStore};
use serde::{Deserialize, Serialize};

use crate::handler::Render;
use crate::home::Home;

/// The reviewable documents of one generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum Document {
    /// The behavioural specification document.
    Spec,
    /// The rebuild design document.
    Design,
}

impl Document {
    const fn label(self) -> &'static str {
        match self {
            Self::Spec => "spec",
            Self::Design => "design",
        }
    }
}

/// Input for `emery show`.
///
/// The input doubles as the verb's clap surface; field docs are its
/// `--help` text.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, clap::Args)]
#[serde(rename_all = "kebab-case")]
pub struct ShowInput {
    /// Reviewable document of the current generation.
    #[arg(value_enum)]
    pub document: Document,
}

/// Successful `emery show` result.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowBody {
    /// Current generation id.
    pub generation: String,
    /// Which document `body` carries.
    pub document: &'static str,
    /// The document bytes.
    pub body: String,
}

// Text mode is the document alone — a deliberate exception to the
// result-line convention so `emery show spec` pipes cleanly; the
// generation id rides the JSON envelope.
impl Render for ShowBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        w.write_all(self.body.as_bytes())
    }
}

impl<P: StateStore + BlobStore> Handler<P> for ShowInput {
    type Error = Error;
    type Output = ShowBody;

    async fn handle(self, context: Context<'_, P>) -> Result<Self::Output, Self::Error> {
        let home = Home::new(context.provider);
        let Some((committed, set)) = home.current_set().await? else {
            return Err(Error::NotFound {
                code: "spec-not-generated".into(),
                description: "spec-not-generated: no specification generation has been committed"
                    .into(),
            });
        };
        let body = match self.document {
            Document::Spec => set.spec,
            Document::Design => set.design,
        };
        Ok(ShowBody {
            generation: committed.id,
            document: self.document.label(),
            body,
        })
    }
}

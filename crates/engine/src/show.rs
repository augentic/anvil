//! The `emery show` operation: a verifiable, non-authoritative
//! projection of the current generation to stdout.

use std::io::Write;

use emery_error::Error;
use omnia_guest::api::Provider;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use omnia_guest::{BlobStore, StateStore};
use serde::{Deserialize, Serialize};

use crate::handler::Render;
use crate::home::Home;

/// The reviewable documents of one generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ShowInput {
    /// Which reviewable document to print.
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

/// The `show` operation route.
#[derive(Clone, Copy, Debug)]
pub struct Show;

impl<P: Provider + StateStore + BlobStore> Operation<P> for Show {
    type Error = crate::handler::Error;
    type Input = ShowInput;
    type Output = ShowBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let home = Home::new(context.provider);
        let Some((committed, set)) = home.current_set().await? else {
            return Err(Error::Diag {
                code: "spec-not-generated",
                detail: "no specification generation has been committed".to_string(),
            }
            .into());
        };
        let body = match input.document {
            Document::Spec => set.spec,
            Document::Design => set.design,
        };
        Ok(ShowBody {
            generation: committed.id,
            document: input.document.label(),
            body,
        })
    }
}

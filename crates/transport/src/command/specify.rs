//! `emery specify` — the reserved spec-generator verb (ADR-0008 §3): a
//! typed stub that parses, then fails `specify-not-implemented`. No
//! orchestration, no output-home scaffolding, no artifacts.

use std::io::Write;

use clap::Args;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::handler::Render;
use serde::{Deserialize, Serialize};

/// Flags for `emery specify` (none yet — the surface is reserved).
#[derive(Debug, Args)]
pub(super) struct SpecifyArgs;

/// Wire input for `emery specify`.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SpecifyInput;

impl TryFrom<SpecifyArgs> for SpecifyInput {
    type Error = error::Error;

    fn try_from(args: SpecifyArgs) -> Result<Self, Self::Error> {
        let SpecifyArgs = args;
        Ok(Self)
    }
}

/// Success body — never produced; the stub always fails typed.
#[derive(Debug, Serialize)]
pub struct SpecifyBody;

impl Render for SpecifyBody {
    fn render(&self, _writer: &mut dyn Write) -> std::io::Result<()> {
        Ok(())
    }
}

/// The reserved `specify` route.
#[derive(Clone, Copy, Debug)]
pub struct Specify;

impl<P: Send + Sync + 'static> Operation<P> for Specify {
    type Error = project::handler::Error;
    type Input = SpecifyInput;
    type Output = SpecifyBody;

    async fn call(
        input: Self::Input, _context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let SpecifyInput = input;
        Err(error::Error::Diag {
            code: "specify-not-implemented",
            detail: "`emery specify` is reserved for the spec generator; it lands with the \
                     remediation programme's Phase 3 walking skeleton"
                .to_string(),
        }
        .into())
    }
}

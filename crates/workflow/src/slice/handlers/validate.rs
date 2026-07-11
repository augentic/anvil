//! `slice validate` — coherence check against the adapter validation
//! rules plus first-use schema validation of per-source `Evidence`
//! files and workflow §Requirement block contract validation of
//! `spec.md` provenance metadata.
//!
//! The pre-adapter gate kernel lives in
//! [`crate::slice::validate`]; this verb orchestrates it against
//! the adapter rules (`artifacts::validate::validate_slice`), returns
//! the report body, and carries the blocking decision on
//! [`crate::handler::Error::Report`] so the transports render findings and the
//! failure envelope on their own channels.

use std::io::Write;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use schema::diagnostics::{Diagnostic, blocking_present};
use serde::{Deserialize, Serialize};

use crate::handler::{Anchor, Ctx, ReportBody};
use crate::slice::validate::{Validation, append_synthesis_journal};

/// Wire input for `slice validate`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ValidateInput {
    /// Slice name.
    pub name: String,
}

/// `specify slice validate <name>`.
#[derive(Clone, Copy, Debug)]
pub struct Validate;

impl<P: Anchor> Operation<P> for Validate {
    type Error = crate::handler::Error;
    type Input = ValidateInput;
    type Output = ReportBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let name = &input.name;
        match crate::slice::validate::run(cx.layout(), name)? {
            Validation::Gate { code, findings } => Err(fail_with(code, findings)),
            Validation::Adapter {
                findings,
                synthesis_tags,
            } => {
                let blocking = blocking_present(&findings);
                let body = ReportBody::new(findings, None, write_finding_row);

                if blocking {
                    Err(crate::handler::Error::report(
                        body,
                        "slice-validation-failed",
                        "slice must satisfy adapter validation",
                        format!("slice `{name}` failed validation"),
                    ))
                } else {
                    // `slice.synthesis.{conflict,divergence,unknown}` emit
                    // once per tagged requirement after a successful validate.
                    append_synthesis_journal(cx.layout(), cx.now(), name, synthesis_tags)?;
                    Ok(body)
                }
            }
        }
    }
}

/// Bundle `findings` with the payload-free [`Error::Validation`] keyed
/// on `code`. Used by every pre-adapter gate so the operator sees the
/// full diagnostic surface before the gate fails the command.
fn fail_with(code: &'static str, findings: Vec<Diagnostic>) -> crate::handler::Error {
    let count = findings.len();
    crate::handler::Error::report(
        ReportBody::new(findings, None, write_finding_row),
        code,
        "slice must satisfy structural invariants",
        format!("{count} blocking finding(s)"),
    )
}

fn write_finding_row(w: &mut dyn Write, finding: &Diagnostic) -> std::io::Result<()> {
    writeln!(w, "  {}", format_finding_line(finding))
}

/// One-line text rendering of a diagnostic for the PASS/FAIL banner.
/// `violation` findings are blocking defects (`[fail]`); `review`
/// findings are deferred requests for judgment (`[review]`).
fn format_finding_line(d: &Diagnostic) -> String {
    let rule = d.rule_id.as_deref().unwrap_or("<unknown>");
    match d.kind {
        schema::diagnostics::DiagnosticKind::Violation => {
            format!("[fail] {}: {}", rule, d.impact)
        }
        schema::diagnostics::DiagnosticKind::Review => {
            format!("[review] {} ({})", rule, d.impact)
        }
    }
}

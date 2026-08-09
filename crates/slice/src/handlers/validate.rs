//! `slice validate` — coherence check over the pre-adapter gates and
//! the adapter validation rules.
//!
//! The blocking decision rides [`project::handler::Error::Report`].

use std::io::Write;

use diagnostics::{Diagnostic, has_blocking};
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::handler::{Anchor, Ctx, ReportBody};
use serde::{Deserialize, Serialize};

use crate::validate::{Validation, append_synthesis_journal};

/// Wire input for `slice validate`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ValidateInput {
    /// Slice name.
    pub name: String,
}

/// `emery slice validate <name>`.
#[derive(Clone, Copy, Debug)]
pub struct Validate;

impl<P: Anchor> Operation<P> for Validate {
    type Error = project::handler::Error;
    type Input = ValidateInput;
    type Output = ReportBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let name = &input.name;
        match crate::validate::run(cx.layout(), name)? {
            Validation::Gate { code, findings } => Err(fail_with(code, findings)),
            Validation::Adapter {
                findings,
                synthesis_tags,
            } => {
                let blocking = has_blocking(&findings);
                let body = ReportBody::new(findings, write_finding_row);

                if blocking {
                    Err(project::handler::Error::report(
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
fn fail_with(code: &'static str, findings: Vec<Diagnostic>) -> project::handler::Error {
    let count = findings.len();
    project::handler::Error::report(
        ReportBody::new(findings, write_finding_row),
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
        diagnostics::DiagnosticKind::Violation => {
            format!("[fail] {}: {}", rule, d.impact)
        }
        diagnostics::DiagnosticKind::Review => {
            format!("[review] {} ({})", rule, d.impact)
        }
    }
}

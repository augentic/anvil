//! `slice validate` — coherence check against the adapter validation
//! rules plus first-use schema validation of per-source `Evidence`
//! files and workflow §Requirement block contract validation of
//! `spec.md` provenance metadata.
//!
//! The pre-adapter gate kernel lives in
//! [`crate::slice::validate`]; this verb orchestrates it against
//! the adapter rules (`artifacts::validate::validate_slice`), returns
//! the report body, and carries the blocking decision on
//! [`crate::verb::Error::Report`] so the transports render findings and the
//! failure envelope on their own channels.

use std::io::Write;

use artifacts::validate::validate_slice;
use error::Error;
use omnia_guest::api::{Context, Handler, Reply};
use schema::diagnostics::{Diagnostic, blocking_present};
use serde::{Deserialize, Serialize};
use crate::slice::validate::{PreAdapter, append_synthesis_journal, pre_adapter_gates};

use crate::verb::{Anchor, Ctx};
use crate::verb::{Out, ReportBody};

/// Wire input for `slice validate`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ValidateInput {
    /// Slice name.
    pub name: String,
}

/// `specify slice validate <name>`.
#[derive(Debug)]
pub struct Validate {
    input: ValidateInput,
}

impl<P: Anchor> Handler<P> for Validate {
    type Error = crate::verb::Error;
    type Input = ValidateInput;
    type Output = Out<ReportBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let name = &self.input.name;
        match pre_adapter_gates(cx.layout(), name)? {
            PreAdapter::Gate { code, findings } => Err(fail_with(code, findings)),
            PreAdapter::Proceed {
                synthesis_tags,
                mut advisories,
            } => {
                // Adapter validation findings — `validate_slice` returns one
                // `violation` diagnostic per structural Fail and one `review`
                // diagnostic per deferred semantic rule. The non-blocking
                // `discovery-lead-synopsis-thin` advisories ride this surface
                // too; only a blocking diagnostic gates exit.
                let mut findings = validate_slice(&cx.slices_dir().join(name))?;
                findings.append(&mut advisories);
                let blocking = blocking_present(&findings);
                let body = ReportBody::new(findings, None, write_finding_row);

                if blocking {
                    Err(crate::verb::Error::Report {
                        body,
                        source: Error::validation_failed(
                            "slice-validation-failed",
                            "slice must satisfy adapter validation",
                            format!("slice `{name}` failed validation"),
                        ),
                    })
                } else {
                    // `slice.synthesis.{conflict,divergence,unknown}` emit
                    // once per tagged requirement after a successful validate.
                    append_synthesis_journal(cx.layout(), cx.now(), name, synthesis_tags)?;
                    Ok(Reply::ok(Out(body)))
                }
            }
        }
    }
}

/// Bundle `findings` with the payload-free [`Error::Validation`] keyed
/// on `code`. Used by every pre-adapter gate so the operator sees the
/// full diagnostic surface before the gate fails the command.
fn fail_with(code: &'static str, findings: Vec<Diagnostic>) -> crate::verb::Error {
    let count = findings.len();
    crate::verb::Error::Report {
        body: ReportBody::new(findings, None, write_finding_row),
        source: Error::validation_failed(
            code,
            "slice must satisfy structural invariants",
            format!("{count} blocking finding(s)"),
        ),
    }
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

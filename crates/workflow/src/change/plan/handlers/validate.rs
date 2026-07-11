//! `plan validate` — structure + plan/change consistency.

use std::io::Write;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use schema::diagnostics::{Diagnostic, blocking, blocking_present};
use serde::{Deserialize, Serialize};

use super::require_file;
use crate::adapter::Resolver;
use crate::change::{Plan, plan_full_report};
use crate::handler::{Anchor, Ctx, ReportBody};

/// Wire input for `plan validate` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct ValidateInput {}

/// `specify plan validate` — structure + plan/change consistency,
/// including the health diagnostics (`cycle-in-depends-on`,
/// `orphan-source`, `stale-workspace-clone`).
#[derive(Clone, Copy, Debug)]
pub struct Validate;

impl<P: Anchor + Resolver> Operation<P> for Validate {
    type Error = crate::handler::Error;
    type Input = ValidateInput;
    type Output = ReportBody;

    async fn call(
        _input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let plan_path = require_file(&cx)?;
        let plan = Plan::load(&plan_path)?;
        let results = plan_full_report(context.provider, &plan, cx.layout());

        let has_errors = blocking_present(&results);
        let body = ReportBody::new(results, Some("Plan OK"), write_row);
        if has_errors {
            Err(crate::handler::Error::report(
                body,
                "plan-structural-errors",
                "plan must be free of structural errors",
                "run 'specify plan validate' for detail",
            ))
        } else {
            Ok(body)
        }
    }
}

fn write_row(w: &mut dyn Write, finding: &Diagnostic) -> std::io::Result<()> {
    let label = if blocking(finding) { "ERROR  " } else { "WARNING" };
    let code = finding.rule_id.as_deref().unwrap_or("<unknown>");
    let entry_col = finding.slice.as_ref().map_or_else(String::new, |e| format!("[{e}]"));
    writeln!(w, "{label} {:<32} {:<24} {}", code, entry_col, finding.impact)
}

//! `emery debt` — the read-only baseline debt projection (RFC-86a
//! D9): the carried `unknown` / `conflict` backlog read from the
//! baseline specs alone.

use std::io::Write;

use artifacts::spec::provenance::RequirementStatus;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::handler::{Anchor, Ctx, Render};
use serde::{Deserialize, Serialize};

use crate::debt::DebtRow;

/// Wire input for `debt` (no fields).
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[expect(
    clippy::empty_structs_with_brackets,
    reason = "serde deserialises the wire `{}` object into a braced struct only"
)]
pub struct DebtInput {}

/// `emery debt`.
///
/// Read-only projection over `.emery/specs/`: emits no journal event
/// and writes nothing. Never joins archived fact logs — every row's
/// reason, origin, originating change, and age come from the
/// self-describing note the merge fold appended (RFC-86a D5).
#[derive(Clone, Copy, Debug)]
pub struct Debt;

impl<P: Anchor> Operation<P> for Debt {
    type Error = project::handler::Error;
    type Input = DebtInput;
    type Output = DebtBody;

    async fn call(
        _input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let rows = crate::debt::baseline(&cx.layout().specs_dir(), cx.now())?;
        Ok(DebtBody { rows })
    }
}

/// Success envelope for `debt`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DebtBody {
    /// Carried rows in domain order then document order.
    pub rows: Vec<DebtRow>,
}

impl DebtBody {
    /// Render the carried rows of one gap kind under `heading` —
    /// conflicts and unknowns get separate blocks (RFC-86a D6: a
    /// shipped-around contradiction is louder news).
    fn render_status(
        &self, w: &mut dyn Write, status: RequirementStatus, heading: &str,
    ) -> std::io::Result<()> {
        let mut headed = false;
        for row in self.rows.iter().filter(|row| row.status == status) {
            if !headed {
                writeln!(w, "  {heading}")?;
                headed = true;
            }
            writeln!(w, "    {}", row.render_line())?;
        }
        Ok(())
    }
}

impl Render for DebtBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.rows.is_empty() {
            return writeln!(w, "baseline debt: none");
        }
        let noun = if self.rows.len() == 1 { "row" } else { "rows" };
        writeln!(w, "baseline debt ({} carried {noun}):", self.rows.len())?;
        self.render_status(w, RequirementStatus::Unknown, "unknown:")?;
        self.render_status(w, RequirementStatus::Conflict, "conflict:")
    }
}

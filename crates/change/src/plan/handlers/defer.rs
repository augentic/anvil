//! `plan defer` — durable, digest-bound gap dispositions (RFC-86a D3).
//!
//! Appends `gap.deferred` / `gap.deferral-retracted` facts keyed by
//! `(slice, digest)`; disposition is recomputed at projection time.

use std::io::Write;

use artifacts::spec::provenance::RequirementStatus;
use error::Error;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::handler::{Anchor, Ctx, Render};
use project::journal::{self, DeferralOrigin, Event, EventKind};
use project::plan::{Disposition, GapRow, Plan, collect_events, plan_gaps_body};
use serde::{Deserialize, Serialize};

use super::require_file;

/// One `<slice>/<req>` selector on `plan defer`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DeferSelector {
    /// Plan entry / slice name.
    pub slice: String,
    /// Requirement id (`REQ-NNN`) — advisory presentation on the fact;
    /// the digest is the durable match key.
    pub req: String,
}

/// Wire input for `plan defer`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DeferInput {
    /// `<slice>/<req>` selectors, at least one.
    pub selectors: Vec<DeferSelector>,
    /// Reason recorded on every appended fact. Required to defer;
    /// optional on retract (a synthesized reason is recorded).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Append `gap.deferral-retracted` instead of `gap.deferred`.
    #[serde(default)]
    pub retract: bool,
}

/// `emery plan defer <slice>/<req>... --reason <text> [--retract]` —
/// the explicit disposition act (RFC-86a D3), one level below
/// `plan drop`: a durable, retractable per-requirement exclusion fact.
///
/// Every selector is validated against the live gap inventory before
/// any fact is appended, so a bad selector in a batch writes nothing.
/// Re-deferring an already-deferred row is legal — the latest fact
/// wins under projection, so the new reason supersedes.
#[derive(Clone, Copy, Debug)]
pub struct Defer;

impl<P: Anchor> Operation<P> for Defer {
    type Error = project::handler::Error;
    type Input = DeferInput;
    type Output = DeferBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let DeferInput {
            selectors,
            reason,
            retract,
        } = input;
        if selectors.is_empty() {
            return Err(
                deferral_invalid("at least one `<slice>/<req>` selector is required").into()
            );
        }
        let reason = resolve_reason(retract, reason.as_deref())?;
        let plan = Plan::load(&require_file(&cx)?)?;
        let events = collect_events(cx.layout())?;
        let inventory = plan_gaps_body(&plan, cx.layout(), &events)?;

        let mut gaps = Vec::with_capacity(selectors.len());
        for selector in &selectors {
            gaps.push(resolve_row(&inventory.rows, selector, retract)?);
        }

        let now = cx.now();
        let facts: Vec<Event> =
            gaps.iter().map(|gap| Event::new(now, fact_kind(gap, retract, &reason))).collect();
        journal::append_batch(cx.layout(), &facts)?;

        Ok(DeferBody {
            action: if retract { DeferAction::Retracted } else { DeferAction::Deferred },
            reason,
            gaps,
        })
    }
}

/// The deferral fact for one resolved row: `origin: operator` on both
/// families — this verb is the explicit operator act (gate-time policy
/// minting carries `origin: policy` instead).
fn fact_kind(gap: &DeferredGap, retract: bool, reason: &str) -> EventKind {
    if retract {
        EventKind::GapDeferralRetracted {
            slice: gap.slice.as_str().into(),
            req: gap.req.clone(),
            requirement_digest: gap.requirement_digest.clone(),
            reason: reason.to_string(),
            origin: DeferralOrigin::Operator,
        }
    } else {
        EventKind::GapDeferred {
            slice: gap.slice.as_str().into(),
            req: gap.req.clone(),
            requirement_digest: gap.requirement_digest.clone(),
            reason: reason.to_string(),
            origin: DeferralOrigin::Operator,
        }
    }
}

/// Resolve the fact reason: required (non-empty) to defer; a retract
/// without `--reason` records the synthesized retraction reason.
fn resolve_reason(retract: bool, reason: Option<&str>) -> Result<String, Error> {
    match reason.map(str::trim).filter(|text| !text.is_empty()) {
        Some(text) => Ok(text.to_string()),
        None if retract => Ok("retracted by operator".to_string()),
        None => Err(deferral_invalid(
            "`--reason` is required to defer — the fact records why the gap waits",
        )),
    }
}

/// Resolve one selector against the projected inventory, enforcing the
/// disposition preconditions (RFC-86a D2 / D3 / D6).
fn resolve_row(
    rows: &[GapRow], selector: &DeferSelector, retract: bool,
) -> Result<DeferredGap, Error> {
    let Some(row) = rows.iter().find(|row| row.slice == selector.slice && row.req == selector.req)
    else {
        return Err(deferral_invalid(format!(
            "no gap row `{}/{}` in the live inventory — `emery plan gaps` lists the in-scope \
             `[unknown]` / `[conflict]` findings",
            selector.slice, selector.req
        )));
    };
    let Some(disposition) = row.disposition else {
        return Err(deferral_invalid(format!(
            "`{}/{}` is `[{}]` — divergence rows are informational and take no disposition",
            selector.slice, selector.req, row.status
        )));
    };
    let Some(digest) = row.requirement_digest.clone() else {
        return Err(deferral_invalid(format!(
            "`{}/{}` carries no requirement digest (legacy `spec.md` inventory) — re-run \
             `emery plan execute` so refine mints `model.yaml` before dispositioning it",
            selector.slice, selector.req
        )));
    };
    if retract && disposition != Disposition::Deferred {
        return Err(deferral_invalid(format!(
            "no live deferral covers `{}/{}` — its disposition is `open`",
            selector.slice, selector.req
        )));
    }
    Ok(DeferredGap {
        slice: row.slice.clone(),
        req: row.req.clone(),
        status: row.status,
        requirement_digest: digest,
    })
}

fn deferral_invalid(detail: impl Into<String>) -> Error {
    Error::validation_failed(
        "plan-deferral-invalid",
        "defer takes open gap rows with `--reason`; `--retract` takes live deferrals",
        detail,
    )
}

/// Which fact family the invocation appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeferAction {
    /// Appended `gap.deferred`.
    Deferred,
    /// Appended `gap.deferral-retracted`.
    Retracted,
}

impl std::fmt::Display for DeferAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Deferred => "deferred",
            Self::Retracted => "retracted",
        })
    }
}

/// One dispositioned requirement in the response, in selector order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DeferredGap {
    /// Plan entry / slice name.
    pub slice: String,
    /// Requirement id at act time (advisory — a re-refine may renumber
    /// it while the digest holds).
    pub req: String,
    /// Typed gap status (`unknown` / `conflict`).
    pub status: RequirementStatus,
    /// Canonical requirement-body digest recorded on the fact — the
    /// durable `(slice, digest)` match key.
    pub requirement_digest: String,
}

/// Success envelope for `plan defer`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct DeferBody {
    /// Which fact family was appended.
    pub action: DeferAction,
    /// Reason recorded on every fact.
    pub reason: String,
    /// One row per selector, in argv order.
    pub gaps: Vec<DeferredGap>,
}

impl Render for DeferBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        for gap in &self.gaps {
            writeln!(w, "{} `{}/{}` [{}]", self.action, gap.slice, gap.req, gap.status)?;
        }
        writeln!(w, "  reason: {}", self.reason)?;
        Ok(())
    }
}

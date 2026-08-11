//! Authorization-epoch open at `plan execute` start: assembles typed
//! `closed-plan` coverage, validates `--waive` selectors against the
//! gap inventory, and appends `plan.execute.started`.

use std::collections::BTreeMap;
use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::Error;
use jiff::Timestamp;
use project::build_record::BuildRecord;
use project::config::Layout;
use project::journal::{self, ClosedPlanCoverage, Event, EventKind, UnknownWaiver};
use project::plan::{Plan, Status, collect_events, in_scope, plan_gaps_body, project_ladders};
use project::slice::SliceMetadata;
use slice::refinement::{self, Freshness};

use crate::plan::wire::load_discovery;

/// One `--waive <slice>/<req>` selector (reason lands separately).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct WaiveSelector {
    /// Plan entry / slice name.
    pub slice: String,
    /// Requirement id (`REQ-NNN`).
    pub req: String,
}

/// Validate CLI waivers against the typed gap inventory (D17).
///
/// Call before acquiring `guest.lock` so a bad `--waive` fails closed
/// without holding the marker.
///
/// # Errors
///
/// `plan-waiver-invalid` when `--reason` / `--waive` pairing is wrong,
/// a selector misses the inventory, or the target finding is not
/// `[unknown]`. Propagates gaps I/O failures.
pub(super) fn validate_waivers(
    layout: Layout<'_>, plan: &Plan, waive: &[WaiveSelector], reason: Option<&str>,
) -> Result<Vec<UnknownWaiver>, Error> {
    let reason = match (waive.is_empty(), reason.map(str::trim)) {
        (true, None | Some("")) => return Ok(Vec::new()),
        (true, Some(_)) => {
            return Err(waiver_invalid("`--reason` requires at least one `--waive <slice>/<req>`"));
        }
        (false, None | Some("")) => {
            return Err(waiver_invalid("`--waive` requires `--reason` (non-empty)"));
        }
        (false, Some(text)) => text.to_string(),
    };

    let gaps = plan_gaps_body(plan, layout)?;
    let mut out = Vec::with_capacity(waive.len());
    for selector in waive {
        let Some(row) =
            gaps.rows.iter().find(|row| row.slice == selector.slice && row.req == selector.req)
        else {
            return Err(waiver_invalid(format!(
                "no open gap for `{}/{}` — waive only an in-scope `[unknown]` requirement",
                selector.slice, selector.req
            )));
        };
        if row.status != artifacts::spec::provenance::RequirementStatus::Unknown {
            return Err(waiver_invalid(format!(
                "`{}/{}` is `{}` — only `[unknown]` may be waived (`[conflict]` is never \
                 waiveable)",
                selector.slice, selector.req, row.status
            )));
        }
        out.push(UnknownWaiver {
            slice: selector.slice.clone(),
            req: selector.req.clone(),
            reason: reason.clone(),
        });
    }
    Ok(out)
}

/// Append `plan.execute.started` for already-validated waivers.
///
/// # Errors
///
/// Propagates coverage-assembly and journal append failures.
pub(super) fn append_started(
    layout: Layout<'_>, plan: &Plan, now: Timestamp, unknown_waivers: Vec<UnknownWaiver>,
) -> Result<(), Error> {
    let coverage = assemble_coverage(layout, plan, unknown_waivers)?;
    let event = Event::new(
        now,
        EventKind::PlanExecuteStarted {
            coverage,
            discovery_digest: None,
        },
    );
    journal::append_one(layout, &event)
}

/// Build `closed-plan` coverage over in-scope leaves (RFC-91 D5).
///
/// Every in-scope leaf **execute may still build** must project a
/// fresh refinement manifest (`slice::refinement::freshness`); its
/// exact refinement digest is covered. A missing or stale manifest
/// fails typed **before** any epoch append — execute never authorizes
/// a refinement that does not exist yet (`refine-under-epoch` is
/// gone). Leaves past their build are not re-litigated: a merged leaf
/// (projected `done`) contributes nothing, and a built leaf parked at
/// merge carries the manifest digest its wave bound at build time —
/// re-running execute is always the resume path.
fn assemble_coverage(
    layout: Layout<'_>, plan: &Plan, unknown_waivers: Vec<UnknownWaiver>,
) -> Result<ClosedPlanCoverage, Error> {
    let plan_bytes = std::fs::read(layout.plan_path())?;
    let plan_digest = format!("sha256:{}", sha256_hex(&plan_bytes));

    let discovery = load_discovery(layout)?;
    let inventory = discovery.as_ref().map_or(&[][..], |d| d.leads());
    let events = collect_events(layout)?;
    let ladders = project_ladders(plan, &events);

    let mut refinements = BTreeMap::new();
    for entry in &plan.entries {
        let slice_dir = layout.slice_dir(entry.name.as_str());
        let meta = load_meta(&slice_dir)?;
        if !in_scope(plan, entry, meta.as_ref()) {
            continue;
        }
        if ladders.get(&entry.name).copied() == Some(Status::Done) {
            continue;
        }
        let name = entry.name.as_str();
        if BuildRecord::present(&slice_dir)
            && let Some(digest) = refinement::file_digest(&slice_dir)?
        {
            // Built, awaiting merge: build promotion may legitimately
            // drift the bundle inputs (`writable-artifacts[]`), so the
            // covered digest is the unchanged manifest the wave bound.
            refinements.insert(name.to_string(), digest.to_string());
            continue;
        }
        match refinement::freshness(layout, plan, entry, inventory)? {
            Freshness::Fresh { digest } => {
                refinements.insert(name.to_string(), digest.to_string());
            }
            Freshness::Missing => {
                return Err(refinement_required(format!(
                    "slice `{name}` has no refinement manifest — run `emery plan refine` \
                     before `emery plan execute`"
                )));
            }
            Freshness::Stale { reasons } => {
                let first = reasons.first().map_or("", String::as_str);
                return Err(refinement_required(format!(
                    "slice `{name}` refinement is stale ({first}) — re-run `emery plan refine` \
                     before `emery plan execute`"
                )));
            }
        }
    }

    Ok(ClosedPlanCoverage::ClosedPlan {
        plan_digest,
        refinements,
        unknown_waivers,
    })
}

const fn refinement_required(detail: String) -> Error {
    Error::Diag {
        code: "plan-refinement-required",
        detail,
    }
}

fn load_meta(slice_dir: &Path) -> Result<Option<SliceMetadata>, Error> {
    match SliceMetadata::load(slice_dir) {
        Ok(meta) => Ok(Some(meta)),
        Err(
            Error::ArtifactNotFound { .. }
            | Error::Diag {
                code: "slice-not-found",
                ..
            },
        ) => Ok(None),
        Err(err) => Err(err),
    }
}

fn waiver_invalid(detail: impl Into<String>) -> Error {
    Error::validation_failed(
        "plan-waiver-invalid",
        "waive only an open `[unknown]` with `--reason`",
        detail,
    )
}

//! Authorization-epoch open at `plan execute` start: assembles typed
//! `closed-plan` coverage, validates `--waive` selectors against the
//! gap inventory, and appends `plan.execute.started`.

use std::collections::BTreeMap;
use std::path::Path;

use diagnostics::digest::sha256_hex;
use error::Error;
use jiff::Timestamp;
use project::config::Layout;
use project::journal::{
    self, ClosedPlanCoverage, Event, EventKind, LeafSpecCoverage, UnknownWaiver,
};
use project::plan::{Plan, dir_cid, in_scope, plan_gaps_body};
use project::slice::SliceMetadata;

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

/// Build `closed-plan` coverage over in-scope leaves.
fn assemble_coverage(
    layout: Layout<'_>, plan: &Plan, unknown_waivers: Vec<UnknownWaiver>,
) -> Result<ClosedPlanCoverage, Error> {
    let plan_bytes = std::fs::read(layout.plan_path())?;
    let plan_digest = format!("sha256:{}", sha256_hex(&plan_bytes));

    let mut specs = BTreeMap::new();
    for entry in &plan.entries {
        let slice_dir = layout.slice_dir(entry.name.as_str());
        let meta = load_meta(&slice_dir)?;
        if !in_scope(plan, entry, meta.as_ref()) {
            continue;
        }
        let leaf = if has_spec_artifacts(&slice_dir) {
            LeafSpecCoverage::Existing {
                digest: dir_cid(&slice_dir.join("specs"))?.to_string(),
            }
        } else {
            LeafSpecCoverage::RefineUnderEpoch
        };
        specs.insert(entry.name.as_str().to_string(), leaf);
    }

    Ok(ClosedPlanCoverage::ClosedPlan {
        plan_digest,
        specs,
        unknown_waivers,
    })
}

fn has_spec_artifacts(slice_dir: &Path) -> bool {
    slice_dir.join("model.yaml").is_file() || slice_dir.join("spec.md").is_file()
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

//! `slice provenance` — project the audit-only provenance view from a
//! slice's single `model.yaml`.
//!
//! Provenance is carried inline in `model.yaml`; this command reshapes
//! it into the per-requirement audit shape on demand. There is no
//! persisted `provenance.yaml`, so the projection cannot drift from the
//! model.

use std::collections::BTreeMap;

use specify_error::{Error, Result};
use specify_model::evidence::ClaimKind;
use specify_workflow::change::Plan;
use specify_workflow::slice::SliceModel;

use crate::runtime::context::Ctx;

/// Generator label stamped on the projection header.
fn generator() -> String {
    format!("specify@{}", env!("CARGO_PKG_VERSION"))
}

/// Resolve the per-slice `authority-override` map from `plan.yaml`.
///
/// Mirrors the slice-entry lookup in `slice validate`: when no plan
/// exists, or the plan carries no entry for `name`, the override map is
/// empty and the provenance projection falls back to the default
/// authority ordering.
fn slice_overrides(ctx: &Ctx, name: &str) -> Result<BTreeMap<ClaimKind, String>> {
    let plan_path = ctx.layout().plan_path();
    if !plan_path.exists() {
        return Ok(BTreeMap::new());
    }
    let plan = Plan::load(&plan_path)?;
    Ok(plan
        .entries
        .iter()
        .find(|e| e.name == name)
        .map(|e| e.authority_override.by_kind.clone())
        .unwrap_or_default())
}

pub(super) fn run(ctx: &Ctx, name: &str) -> Result<()> {
    let slice_dir = ctx.slices_dir().join(name);
    let model_path = slice_dir.join("model.yaml");
    if !model_path.is_file() {
        return Err(Error::validation_failed(
            "slice-model-missing",
            "a synthesized slice carries model.yaml",
            format!(
                "slice `{name}` has no model.yaml at {}; run `specify slice synthesize {name}` first",
                model_path.display()
            ),
        ));
    }
    let model = SliceModel::load(&model_path)?;
    let overrides = slice_overrides(ctx, name)?;
    let index = model.to_provenance_index(&slice_dir, &overrides, ctx.now(), generator())?;

    ctx.write(&index, |w, index| {
        writeln!(w, "slice: {}", index.slice)?;
        for req in &index.requirements {
            writeln!(
                w,
                "  {} [{}] {} ({} claim(s))",
                req.id,
                req.status,
                req.resolution,
                req.contributing_claims.len()
            )?;
        }
        Ok(())
    })
}

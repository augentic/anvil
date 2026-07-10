//! `slice provenance` — project the audit-only provenance view from a
//! slice's single `model.yaml`.
//!
//! Provenance is carried inline in `model.yaml`; this verb reshapes it
//! into the per-requirement audit shape on demand. There is no
//! persisted `provenance.yaml`, so the projection cannot drift from the
//! model.

use std::collections::BTreeMap;
use std::io::Write;

use artifacts::evidence::ClaimKind;
use error::{Error, Result};
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use crate::change::Plan;
use crate::handler::{Anchor, Ctx, Render};
use crate::slice::SliceModel;
use crate::slice::provenance::ProvenanceIndex;

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
fn slice_overrides(cx: &Ctx, name: &str) -> Result<BTreeMap<ClaimKind, String>> {
    let plan_path = cx.layout().plan_path();
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

/// Wire input for `slice provenance`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ProvenanceInput {
    /// Slice name.
    pub name: String,
}

/// `specify slice provenance <name>`.
#[derive(Clone, Copy, Debug)]
pub struct Provenance;

impl<P: Anchor> Operation<P> for Provenance {
    type Error = crate::handler::Error;
    type Input = ProvenanceInput;
    type Output = ProvenanceIndex;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let name = &input.name;
        let slice_dir = cx.slices_dir().join(name);
        let model_path = slice_dir.join("model.yaml");
        if !model_path.is_file() {
            return Err(Error::validation_failed(
                "slice-model-missing",
                "a synthesized slice carries model.yaml",
                format!(
                    "slice `{name}` has no model.yaml at {}; run `specify slice refine {name}` \
                     first",
                    model_path.display()
                ),
            )
            .into());
        }
        let model = SliceModel::load(&model_path)?;
        let overrides = slice_overrides(&cx, name)?;
        let index = model.to_provenance_index(&slice_dir, &overrides, cx.now(), generator())?;
        Ok(index)
    }
}

impl Render for ProvenanceIndex {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "slice: {}", self.slice)?;
        for req in &self.requirements {
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
    }
}

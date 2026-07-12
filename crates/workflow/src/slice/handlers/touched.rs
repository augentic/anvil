//! `slice touched-specs` and `slice overlap`.

use std::io::Write;

use error::{Error, Result};
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use crate::handler::{Anchor, Ctx, Render};
use crate::merge::{MergeStrategy, artifact_classes};
use crate::slice::{
    Overlap as SliceOverlap, SliceMetadata, SpecKind, TouchedSpec, actions as slice_actions,
};

// ---------------------------------------------------------------------------
// slice touched-specs
// ---------------------------------------------------------------------------

/// Wire input for `slice touched-specs`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct TouchedSpecsInput {
    /// Slice name.
    pub name: String,
    /// Scan `specs/` subdirs and classify each as new or modified.
    #[serde(default)]
    pub scan: bool,
    /// Replace `touched_specs` with the listed adapters (each
    /// `<name>:new|modified`).
    #[serde(default)]
    pub set: Vec<String>,
}

/// `specify slice touched-specs <name>` — scan or overwrite
/// `touched_specs` on `metadata.yaml`.
#[derive(Clone, Copy, Debug)]
pub struct TouchedSpecs;

impl<P: Anchor> Operation<P> for TouchedSpecs {
    type Error = crate::handler::Error;
    type Input = TouchedSpecsInput;
    type Output = SpecsBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let TouchedSpecsInput { name, scan, set } = input;
        let slice_dir = cx.layout().slice_dir(&name);

        let entries = if !set.is_empty() {
            let v = parse_touched_spec_set(&set)?;
            let metadata = slice_actions::write_touched(&slice_dir, v)?;
            metadata.touched_specs
        } else if scan {
            // Classifies a delta as `new` vs `modified` against the omnia
            // ThreeWayMerge baseline. Reach through the omnia synthesiser
            // so any future change to the baseline location flows through
            // one place.
            let classes = artifact_classes(&cx.project_dir, &slice_dir);
            let baseline_dir = classes
                .iter()
                .find(|c| matches!(c.strategy, MergeStrategy::ThreeWayMerge))
                .map_or_else(
                    || cx.layout().specify_dir().join("specs"),
                    |c| c.baseline_dir.clone(),
                );
            let scanned = slice_actions::scan_touched(&slice_dir, &baseline_dir)?;
            let metadata = slice_actions::write_touched(&slice_dir, scanned)?;
            metadata.touched_specs
        } else {
            let metadata = SliceMetadata::load(&slice_dir)?;
            metadata.touched_specs
        };

        Ok(SpecsBody {
            name,
            touched_specs: entries,
        })
    }
}

/// Success envelope for `slice touched-specs`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct SpecsBody {
    /// Slice name.
    pub name: String,
    /// The touched-specs rows as persisted.
    pub touched_specs: Vec<TouchedSpec>,
}

impl Render for SpecsBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.touched_specs.is_empty() {
            return writeln!(w, "{}: no touched specs", self.name);
        }
        writeln!(w, "{}:", self.name)?;
        for entry in &self.touched_specs {
            writeln!(w, "  {} ({})", entry.name, entry.kind)?;
        }
        Ok(())
    }
}

fn parse_touched_spec_set(raw: &[String]) -> Result<Vec<TouchedSpec>> {
    let mut out: Vec<TouchedSpec> = Vec::with_capacity(raw.len());
    for entry in raw {
        let (name, kind) = entry.split_once(':').ok_or_else(|| Error::Diag {
            code: "touched-specs-entry-malformed",
            detail: format!(
                "touched-specs entry `{entry}` must be `<name>:new` or `<name>:modified`",
            ),
        })?;
        let kind = match kind {
            "new" => SpecKind::New,
            "modified" => SpecKind::Modified,
            other => {
                return Err(Error::Diag {
                    code: "touched-specs-kind-invalid",
                    detail: format!("touched-specs kind `{other}` must be `new` or `modified`"),
                });
            }
        };
        out.push(TouchedSpec {
            name: name.to_string(),
            kind,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

// ---------------------------------------------------------------------------
// slice overlap
// ---------------------------------------------------------------------------

/// Wire input for `slice overlap`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct OverlapInput {
    /// Slice name.
    pub name: String,
}

/// `specify slice overlap <name>` — report overlapping
/// `touched_specs` with other active slices.
#[derive(Clone, Copy, Debug)]
pub struct Overlap;

impl<P: Anchor> Operation<P> for Overlap {
    type Error = crate::handler::Error;
    type Input = OverlapInput;
    type Output = OverlapBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let name = input.name;
        let slices_dir = cx.layout().slices_dir();
        let overlaps = slice_actions::overlap(&slices_dir, &name)?;

        Ok(OverlapBody { name, overlaps })
    }
}

/// Success envelope for `slice overlap`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct OverlapBody {
    /// Slice name.
    pub name: String,
    /// Overlapping capability rows.
    pub overlaps: Vec<SliceOverlap>,
}

impl Render for OverlapBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.overlaps.is_empty() {
            return writeln!(w, "{}: no overlapping slices", self.name);
        }
        for o in &self.overlaps {
            writeln!(
                w,
                "{}: also touched by `{}` ({} vs {})",
                o.capability, o.other_slice, o.our_spec_type, o.other_spec_type,
            )?;
        }
        Ok(())
    }
}

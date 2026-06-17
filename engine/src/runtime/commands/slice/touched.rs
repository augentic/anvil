//! `slice touched-specs` and `slice overlap`.

use std::io::Write;

use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::merge::MergeStrategy;
use specify_workflow::slice::{
    Overlap, SliceMetadata, SpecKind, TouchedSpec, actions as slice_actions,
};

use super::artifact_classes;
use crate::runtime::context::Ctx;

pub(super) fn specs(ctx: &Ctx, name: String, scan: bool, set: &[String]) -> Result<()> {
    let slice_dir = ctx.slices_dir().join(&name);

    let entries = if !set.is_empty() {
        let v = parse_touched_spec_set(set)?;
        let metadata = slice_actions::write_touched(&slice_dir, v)?;
        metadata.touched_specs
    } else if scan {
        // Classifies a delta as `new` vs `modified` against the omnia
        // ThreeWayMerge baseline. Reach through the omnia synthesiser
        // so any future change to the baseline location flows through
        // one place.
        let classes = artifact_classes(&ctx.project_dir, &slice_dir);
        let baseline_dir = classes
            .iter()
            .find(|c| matches!(c.strategy, MergeStrategy::ThreeWayMerge))
            .map_or_else(|| ctx.layout().specify_dir().join("specs"), |c| c.baseline_dir.clone());
        let scanned = slice_actions::scan_touched(&slice_dir, &baseline_dir)?;
        let metadata = slice_actions::write_touched(&slice_dir, scanned)?;
        metadata.touched_specs
    } else {
        let metadata = SliceMetadata::load(&slice_dir)?;
        metadata.touched_specs
    };

    ctx.write(
        &SpecsBody {
            name,
            touched_specs: entries,
        },
        write_specs_text,
    )?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct SpecsBody {
    name: String,
    touched_specs: Vec<TouchedSpec>,
}

fn write_specs_text(w: &mut dyn Write, body: &SpecsBody) -> std::io::Result<()> {
    if body.touched_specs.is_empty() {
        return writeln!(w, "{}: no touched specs", body.name);
    }
    writeln!(w, "{}:", body.name)?;
    for entry in &body.touched_specs {
        writeln!(w, "  {} ({})", entry.name, entry.kind)?;
    }
    Ok(())
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

pub(super) fn overlap(ctx: &Ctx, name: String) -> Result<()> {
    let slices_dir = ctx.slices_dir();
    let overlaps = slice_actions::overlap(&slices_dir, &name)?;

    ctx.write(
        &OverlapBody {
            name,
            overlaps: &overlaps,
        },
        write_overlap_text,
    )?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct OverlapBody<'a> {
    name: String,
    overlaps: &'a [Overlap],
}

fn write_overlap_text(w: &mut dyn Write, body: &OverlapBody<'_>) -> std::io::Result<()> {
    if body.overlaps.is_empty() {
        return writeln!(w, "{}: no overlapping slices", body.name);
    }
    for o in body.overlaps {
        writeln!(
            w,
            "{}: also touched by `{}` ({} vs {})",
            o.capability, o.other_slice, o.our_spec_type, o.other_spec_type,
        )?;
    }
    Ok(())
}

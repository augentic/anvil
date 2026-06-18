//! `slice model show` — read-only viewer over a slice's single
//! `model.yaml`.
//!
//! `--format json` serialises the persisted [`SliceModel`] verbatim;
//! text renders a concise header + requirement + task summary. The
//! command never writes — it mirrors `slice provenance`'s load + render
//! shape, including the `slice-model-missing` error for an absent model.

use std::io::Write;

use specify_error::{Error, Result};
use specify_workflow::slice::SliceModel;

use crate::runtime::context::Ctx;

pub(super) fn show(ctx: &Ctx, name: &str) -> Result<()> {
    let model_path = ctx.slices_dir().join(name).join("model.yaml");
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

    ctx.write(&model, render_text)
}

/// Concise human view: a header line, one line per requirement, then
/// one line per task.
fn render_text(w: &mut dyn Write, model: &SliceModel) -> std::io::Result<()> {
    let slice = model.slice.as_deref().unwrap_or("<unnamed>");
    let project = model.project.as_deref().unwrap_or("<none>");
    let version = model.version.map_or_else(|| "<none>".to_string(), |v| v.to_string());
    writeln!(w, "slice: {slice}  project: {project}  version: {version}")?;

    writeln!(w, "requirements ({}):", model.requirements.len())?;
    for req in &model.requirements {
        let id = req.id.as_deref().unwrap_or("REQ-???");
        let status = req.status.map_or_else(|| "?".to_string(), |s| s.to_string());
        write!(w, "  {id} [{status}] {}", req.title)?;
        if !req.sources.is_empty() {
            write!(w, " — sources: {}", req.sources.join(", "))?;
        }
        writeln!(w)?;
    }

    writeln!(w, "tasks ({}):", model.tasks.len())?;
    for task in &model.tasks {
        write!(w, "  {} {}", task.id, task.text)?;
        if !task.satisfies.is_empty() {
            write!(w, " — satisfies: {}", task.satisfies.join(", "))?;
        }
        if !task.depends_on.is_empty() {
            write!(w, " — depends-on: {}", task.depends_on.join(", "))?;
        }
        writeln!(w)?;
    }
    Ok(())
}

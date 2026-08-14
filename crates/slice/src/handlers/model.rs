//! `slice model show` — read-only viewer over a slice's `model.yaml`.
//!
//! JSON serialises the persisted [`SliceModel`] verbatim; text renders
//! a concise summary. The verb never writes.

use std::io::Write;

use error::Error;
use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use project::handler::{Anchor, Ctx, Render};
use serde::{Deserialize, Serialize};

use crate::SliceModel;

/// Wire input for `slice model show`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ModelShowInput {
    /// Slice name.
    pub name: String,
}

/// `emery slice model show <name>`.
#[derive(Clone, Copy, Debug)]
pub struct ModelShow;

impl<P: Anchor> Operation<P> for ModelShow {
    type Error = project::handler::Error;
    type Input = ModelShowInput;
    type Output = SliceModel;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let name = &input.name;
        let slice_dir = cx.layout().slice_dir(name);
        // A missing directory is a typo'd name (`slice-not-found`),
        // not an unsynthesized slice — don't advise a refine run.
        if !slice_dir.is_dir() {
            return Err(project::slice::slice_not_found(&slice_dir).into());
        }
        let model_path = slice_dir.join("model.yaml");
        if !model_path.is_file() {
            return Err(Error::validation_failed(
                "slice-model-missing",
                "a synthesized slice carries model.yaml",
                format!(
                    "slice `{name}` has no model.yaml at {}; run `emery plan execute` \
                     first",
                    model_path.display()
                ),
            )
            .into());
        }
        let model = SliceModel::load(&model_path)?;
        Ok(model)
    }
}

/// Concise human view: a header line, one line per requirement, then
/// one line per task.
impl Render for SliceModel {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        let slice = self.slice.as_deref().unwrap_or("<unnamed>");
        let target = self.target.as_deref().unwrap_or("<none>");
        let version = self.version.map_or_else(|| "<none>".to_string(), |v| v.to_string());
        writeln!(w, "slice: {slice}  target: {target}  version: {version}")?;

        writeln!(w, "requirements ({}):", self.requirements.len())?;
        for req in &self.requirements {
            let id = req.id.as_deref().unwrap_or("REQ-???");
            let status = req.status.map_or_else(|| "?".to_string(), |s| s.to_string());
            write!(w, "  {id} [{status}] {}", req.title)?;
            if !req.sources.is_empty() {
                write!(w, " — sources: {}", req.sources.join(", "))?;
            }
            writeln!(w)?;
        }

        writeln!(w, "tasks ({}):", self.tasks.len())?;
        for task in &self.tasks {
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
}

//! `slice merge preview | conflict-check`. Owns the merge-side JSON
//! DTOs and summarisers; `slice merge run` itself is an orchestration
//! verb ([`crate::orchestrate::MergeRun`]).

use std::io::Write;

use omnia_guest::api::{Context, Handler, Reply};
use serde::{Deserialize, Serialize};
use crate::merge::{
    BaselineConflict, MergeOperation, MergePreviewEntry, OpaqueAction, conflict_check, slice,
    summarise_operations,
};

use super::artifact_classes;
use crate::verb::{Anchor, Ctx};
use crate::verb::{Out, Render};

// ---------------------------------------------------------------------------
// slice merge preview
// ---------------------------------------------------------------------------

/// Wire input for `slice merge preview`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PreviewInput {
    /// Slice name.
    pub name: String,
}

/// `specify slice merge preview <name>` — show the merge operations
/// that would be applied, without writing.
#[derive(Debug)]
pub struct Preview {
    input: PreviewInput,
}

impl<P: Anchor> Handler<P> for Preview {
    type Error = crate::verb::Error;
    type Input = PreviewInput;
    type Output = Out<PreviewBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let slice_dir = cx.slices_dir().join(&self.input.name);
        let classes = artifact_classes(&cx.project_dir, &slice_dir);
        let result = slice::preview(&slice_dir, &classes)?;

        // The JSON preview surface keeps its `specs` and `contracts` arrays
        // by grouping the engine's class-tagged entries by their `class_name`.
        // The literal output keys live here — alongside the omnia-default
        // synthesiser — rather than in the engine.
        let specs: Vec<MergePreviewEntry> =
            result.three_way.into_iter().filter(|e| e.class_name == "specs").collect();
        let contracts: Vec<ContractItem> = result
            .opaque
            .iter()
            .filter(|e| e.class_name == "contracts")
            .map(|entry| ContractItem {
                path: entry.relative_path.clone(),
                action: entry.action,
            })
            .collect();

        Ok(Reply::ok(Out(PreviewBody {
            slice_dir: slice_dir.display().to_string(),
            specs,
            contracts,
        })))
    }
}

/// Success envelope for `slice merge preview`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PreviewBody {
    /// Display path of the slice directory.
    pub slice_dir: String,
    /// Three-way merge previews for the `specs` class.
    pub specs: Vec<MergePreviewEntry>,
    /// Opaque contract changes.
    pub contracts: Vec<ContractItem>,
}

impl Render for PreviewBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.specs.is_empty() {
            writeln!(w, "No delta specs to merge.")?;
        } else {
            for entry in &self.specs {
                writeln!(w, "{}: {}", entry.name, summarise_operations(&entry.result.operations))?;
                for op in &entry.result.operations {
                    writeln!(w, "  {}", operation_label(op))?;
                }
            }
        }
        if !self.contracts.is_empty() {
            writeln!(w, "\nContract changes:")?;
            for c in &self.contracts {
                let (sigil, label) = match c.action {
                    OpaqueAction::Added => ("+", "added"),
                    OpaqueAction::Replaced => ("~", "replaced"),
                };
                writeln!(w, "  {sigil} contracts/{} ({label})", c.path)?;
            }
        }
        Ok(())
    }
}

/// One opaque contract change in a preview.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ContractItem {
    /// Path relative to the slice's `contracts/` tree.
    pub path: String,
    /// Added or replaced.
    pub action: OpaqueAction,
}

// ---------------------------------------------------------------------------
// slice merge conflict-check
// ---------------------------------------------------------------------------

/// Wire input for `slice merge conflict-check`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConflictCheckInput {
    /// Slice name.
    pub name: String,
}

/// `specify slice merge conflict-check <name>` — report `type:
/// modified` baselines modified after this slice's `defined_at`.
#[derive(Debug)]
pub struct ConflictCheck {
    input: ConflictCheckInput,
}

impl<P: Anchor> Handler<P> for ConflictCheck {
    type Error = crate::verb::Error;
    type Input = ConflictCheckInput;
    type Output = Out<ConflictCheckBody>;

    fn from_input(input: Self::Input) -> Result<Self, Self::Error> {
        Ok(Self { input })
    }

    async fn handle(self, ctx: Context<'_, P>) -> Result<Reply<Self::Output>, Self::Error> {
        let cx = Ctx::load(ctx.provider)?;
        let slice_dir = cx.slices_dir().join(&self.input.name);
        let classes = artifact_classes(&cx.project_dir, &slice_dir);
        let conflicts = conflict_check(&slice_dir, &classes)?;

        Ok(Reply::ok(Out(ConflictCheckBody {
            slice_dir: slice_dir.display().to_string(),
            conflicts,
        })))
    }
}

/// Success envelope for `slice merge conflict-check`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConflictCheckBody {
    /// Display path of the slice directory.
    pub slice_dir: String,
    /// Baselines modified after this slice's `defined_at`.
    pub conflicts: Vec<BaselineConflict>,
}

impl Render for ConflictCheckBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        if self.conflicts.is_empty() {
            return writeln!(w, "No baseline conflicts.");
        }
        for c in &self.conflicts {
            writeln!(
                w,
                "{}: baseline modified {} (defined_at {})",
                c.adapter,
                c.baseline_modified_at.strftime("%Y-%m-%dT%H:%M:%SZ"),
                c.defined_at,
            )?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// MergeOperation rendering.
// ---------------------------------------------------------------------------

fn operation_label(op: &MergeOperation) -> String {
    match op {
        MergeOperation::Added { id, name } => format!("ADDING: {id} — {name}"),
        MergeOperation::Modified { id, name } => format!("MODIFYING: {id} — {name}"),
        MergeOperation::Removed { id, name } => format!("REMOVING: {id} — {name}"),
        MergeOperation::Renamed {
            id,
            old_name,
            new_name,
        } => format!("RENAMING: {id} — {old_name} -> {new_name}"),
        MergeOperation::CreatedBaseline { requirement_count } => {
            format!("CREATING baseline with {requirement_count} requirement(s)")
        }
    }
}

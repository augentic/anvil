//! `slice merge run | preview | conflict-check`. Owns the merge-side
//! JSON DTOs and summarisers; `run` drives the deterministic
//! [`crate::orchestrate::merge`] kernel.

use std::io::Write;
use std::path::PathBuf;

use omnia_guest::api::invoke::CallContext;
use omnia_guest::api::operation::Operation;
use serde::{Deserialize, Serialize};

use crate::handler::{Anchor, Ctx, Render};
use crate::merge::{
    BaselineConflict, MergeOperation, OpaqueAction, PreviewEntry, artifact_classes, conflict_check,
    slice, summarise_operations,
};
use crate::orchestrate;

/// Wire input for `slice merge run`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct MergeRunInput {
    /// Slice to merge.
    pub name: String,
    /// Authorise a whole-document composition overwrite.
    #[serde(default)]
    pub allow_composition_replace: bool,
}

/// `specify slice merge run <name>` → the internal merge orchestration
/// (deterministic-only — no target merge brief is dispatched).
#[derive(Clone, Copy, Debug)]
pub struct MergeRun;

impl<P: Anchor> Operation<P> for MergeRun {
    type Error = crate::handler::Error;
    type Input = MergeRunInput;
    type Output = MergeBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let MergeRunInput {
            name,
            allow_composition_replace,
        } = input;
        let outcome = orchestrate::merge(cx.layout(), cx.now(), &name, allow_composition_replace)?;
        Ok(MergeBody {
            slice: name,
            merged: outcome.merged.into_iter().map(|entry| entry.name).collect(),
            decisions: outcome.decisions,
            archive_path: outcome.archive_path,
        })
    }
}

/// Success envelope for `slice merge run`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct MergeBody {
    /// Merged slice.
    pub slice: String,
    /// Updated baseline specs.
    pub merged: Vec<String>,
    /// Promoted Decision Records.
    pub decisions: Vec<String>,
    /// Archived slice location.
    pub archive_path: PathBuf,
}

impl Render for MergeBody {
    fn render(&self, w: &mut dyn Write) -> std::io::Result<()> {
        writeln!(w, "merged {}", self.slice)?;
        for name in &self.merged {
            writeln!(w, "spec: {name}")?;
        }
        for decision in &self.decisions {
            writeln!(w, "decision: {decision}")?;
        }
        writeln!(w, "archived: {}", self.archive_path.display())
    }
}

/// Wire input for `slice merge preview`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct PreviewInput {
    /// Slice to preview.
    pub name: String,
}

/// `specify slice merge preview <name>` — show the merge operations
/// that would be applied, without writing.
#[derive(Clone, Copy, Debug)]
pub struct Preview;

impl<P: Anchor> Operation<P> for Preview {
    type Error = crate::handler::Error;
    type Input = PreviewInput;
    type Output = PreviewBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let slice_dir = cx.layout().slice_dir(&input.name);
        let classes = artifact_classes(&cx.project_dir, &slice_dir);
        let result = slice::preview(&slice_dir, &classes)?;

        // The JSON preview surface keeps its `specs` and `contracts` arrays
        // by grouping the engine's class-tagged entries by their `class_name`.
        // The literal output keys live here — alongside the omnia-default
        // synthesiser — rather than in the engine.
        let specs: Vec<PreviewEntry> =
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

        Ok(PreviewBody {
            slice_dir,
            specs,
            contracts,
        })
    }
}

/// Success envelope for `slice merge preview`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct PreviewBody {
    /// Previewed slice location.
    pub slice_dir: PathBuf,
    /// Three-way spec operations.
    pub specs: Vec<PreviewEntry>,
    /// Opaque contract operations.
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

/// Wire input for `slice merge conflict-check`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConflictCheckInput {
    /// Slice to inspect.
    pub name: String,
}

/// `specify slice merge conflict-check <name>` — report `type:
/// modified` baselines modified after this slice's `defined_at`.
#[derive(Clone, Copy, Debug)]
pub struct ConflictCheck;

impl<P: Anchor> Operation<P> for ConflictCheck {
    type Error = crate::handler::Error;
    type Input = ConflictCheckInput;
    type Output = ConflictCheckBody;

    async fn call(
        input: Self::Input, context: CallContext<'_, P>,
    ) -> Result<Self::Output, Self::Error> {
        let cx = Ctx::load(context.provider)?;
        let slice_dir = cx.layout().slice_dir(&input.name);
        let classes = artifact_classes(&cx.project_dir, &slice_dir);
        let conflicts = conflict_check(&slice_dir, &classes)?;

        Ok(ConflictCheckBody { slice_dir, conflicts })
    }
}

/// Success envelope for `slice merge conflict-check`.
#[derive(Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct ConflictCheckBody {
    /// Inspected slice location.
    pub slice_dir: PathBuf,
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

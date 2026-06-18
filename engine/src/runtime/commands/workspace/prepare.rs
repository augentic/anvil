use std::io::Write;
use std::path::PathBuf;

use serde::Serialize;
use specify_error::{Error, Result};
use specify_workflow::registry::Registry;
use specify_workflow::registry::branch::{
    Prepared, Request as BranchRequest, prepare as prepare_branch,
};

use super::registry_missing;
use crate::runtime::context::Ctx;

pub fn prepare(
    ctx: &Ctx, project: &str, change: String, sources: Vec<PathBuf>, outputs: Vec<PathBuf>,
) -> Result<()> {
    let Some(registry) = Registry::load(&ctx.project_dir)? else {
        return Err(registry_missing());
    };
    let project_filter = [project.to_string()];
    let selected = registry.select(&project_filter)?;
    let Some(project) = selected.first() else {
        return Err(Error::Diag {
            code: "workspace-prepare-no-project",
            detail: "workspace prepare resolved no project".to_string(),
        });
    };
    let request = BranchRequest {
        change_name: change,
        source_paths: sources,
        output_paths: outputs,
    };

    match prepare_branch(&ctx.project_dir, project, &request) {
        Ok(prepared) => {
            ctx.write(
                &PrepareBody {
                    prepared: true,
                    inner: &prepared,
                },
                write_prepare_text,
            )?;
            Ok(())
        }
        Err(diagnostic) => Err(Error::BranchPrepareFailed {
            project: project.name.clone(),
            key: diagnostic.key,
            detail: diagnostic.message,
            paths: diagnostic.paths,
        }),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct PrepareBody<'a> {
    prepared: bool,
    #[serde(flatten)]
    inner: &'a Prepared,
}

fn write_prepare_text(w: &mut dyn Write, body: &PrepareBody<'_>) -> std::io::Result<()> {
    let p = body.inner;
    writeln!(
        w,
        "workspace branch prepared: {} {} ({:?}, {:?})",
        p.project, p.branch, p.local_branch, p.remote_branch
    )?;
    if !p.dirty.tracked_allowed.is_empty() || !p.dirty.untracked.is_empty() {
        writeln!(
            w,
            "dirty: {} tracked resume-safe, {} untracked",
            p.dirty.tracked_allowed.len(),
            p.dirty.untracked.len()
        )?;
    }
    Ok(())
}

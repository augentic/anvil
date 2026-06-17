use std::io::Write;

use serde::Serialize;
use specify_error::Result;
use specify_workflow::journal;
use specify_workflow::registry::Registry;
use specify_workflow::registry::workspace::{regenerate_topology_lock, sync_projects};

use super::registry_missing;
use crate::runtime::context::Ctx;

pub fn sync(ctx: &Ctx, projects: &[String]) -> Result<()> {
    let registry = Registry::load(&ctx.project_dir)?;
    let synced = if let Some(reg) = registry.as_ref() {
        let selected = reg.select(projects)?;
        sync_projects(&ctx.project_dir, &selected)?;
        // Project the materialised slots' `project.yaml` topology
        // facets into the committed `.specify/topology.lock`.
        regenerate_topology_lock(&ctx.project_dir, reg)?;
        // workflow §Observability: one `workspace.sync.completed` per
        // successful sync; the registry-less no-op path emits nothing.
        let event = journal::Event::new(
            ctx.now(),
            journal::EventKind::WorkspaceSyncCompleted {
                projects: selected.iter().map(|p| p.name.clone()).collect(),
            },
        );
        journal::append_batch(ctx.layout(), std::slice::from_ref(&event))?;
        true
    } else if !projects.is_empty() {
        return Err(registry_missing());
    } else {
        false
    };
    let message = (!synced).then_some("no registry declared at registry.yaml; nothing to sync");
    ctx.write(
        &SyncBody {
            registry,
            synced,
            message,
        },
        write_sync_text,
    )?;
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "kebab-case")]
struct SyncBody {
    registry: Option<Registry>,
    synced: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    message: Option<&'static str>,
}

fn write_sync_text(w: &mut dyn Write, body: &SyncBody) -> std::io::Result<()> {
    if body.synced {
        writeln!(w, "workspace sync complete")
    } else {
        writeln!(w, "no registry declared at registry.yaml; nothing to sync")
    }
}

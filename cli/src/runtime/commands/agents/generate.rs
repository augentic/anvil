//! Init-time fenced AGENTS.md writer.

use specify_error::{Error, Result};
use specify_model::atomic::bytes_write;
use specify_workflow::agents::{fences, fingerprint, lock};

use super::{context_lock_path, error_from_fence, read_optional, render_document};
use crate::runtime::context::Ctx;

pub fn for_init(ctx: &Ctx) -> Result<Outcome> {
    let body = body(ctx)?;
    Ok(Outcome {
        changed: body.changed,
        disposition: body.disposition,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    pub changed: bool,
    pub disposition: &'static str,
}

fn body(ctx: &Ctx) -> Result<Body> {
    let (generated, context_fingerprint) = render_document(ctx)?;
    let expected_lock = lock::ContextLock::from_fingerprint(&context_fingerprint);
    let lock_path = context_lock_path(ctx);
    let existing_lock = lock::load(&lock_path)?;
    let agents_path = ctx.project_dir.join("AGENTS.md");
    let existing = read_optional(&agents_path)?;
    refuse_modified_fenced_body(existing.as_deref(), existing_lock.as_ref())?;
    let planned = fences::plan_agents_write(existing.as_deref(), generated.as_bytes(), false)
        .map_err(error_from_fence)?;
    let agents_changed = planned.disposition != fences::WriteDisposition::Unchanged;
    let lock_changed = existing_lock.as_ref() != Some(&expected_lock);
    let changed = agents_changed || lock_changed;

    if agents_changed {
        bytes_write(&agents_path, &planned.bytes)?;
    }
    if lock_changed {
        lock::save(&lock_path, &expected_lock)?;
    }

    let disposition = match planned.disposition {
        fences::WriteDisposition::Create => "create",
        fences::WriteDisposition::ForceRewriteUnfenced => "force-rewrite-unfenced",
        fences::WriteDisposition::ReplaceFencedBlock => "replace-fenced-block",
        fences::WriteDisposition::Unchanged => "unchanged",
    };
    Ok(Body { changed, disposition })
}

struct Body {
    changed: bool,
    disposition: &'static str,
}

fn refuse_modified_fenced_body(
    agents: Option<&[u8]>, existing_lock: Option<&lock::ContextLock>,
) -> Result<()> {
    let (Some(agents), Some(existing_lock)) = (agents, existing_lock) else {
        return Ok(());
    };
    let Some(current) = fences::parse_document(agents).map_err(error_from_fence)? else {
        return Ok(());
    };
    let actual_body = fingerprint::body_sha256(current.body());
    if actual_body != existing_lock.fences.body_sha256 {
        return Err(Error::Diag {
            code: "context-fenced-content-modified",
            detail: "AGENTS.md drifted from .specify/context.lock".to_string(),
        });
    }
    Ok(())
}

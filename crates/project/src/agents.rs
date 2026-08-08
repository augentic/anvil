//! Init-time `AGENTS.md` scaffold.
//!
//! Writes a short fenced context document plus a minimal
//! `.emery/context.lock` when `AGENTS.md` is absent.

use std::io::ErrorKind;

use artifacts::atomic::{bytes_write, yaml_write};
use error::Error;
use serde::Serialize;

use crate::adapter::{AdapterSelector, Resolver};
use crate::config::{Layout, ProjectConfig, is_slot};
use crate::handler::ExecutionPaths;

/// Skip reason when init-time context generation did not run.
///
/// Projected onto the init body's `context-skip-reason` wire field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skip {
    /// Root `AGENTS.md` already exists; init preserves it byte-for-byte.
    ExistingAgentsMd,
    /// The project is a materialised `workspace/<peer>/` slot; slots
    /// inherit context from their owning project.
    WorkspaceClone,
}

impl Skip {
    /// The kebab-case wire token (`existing-agents-md` / `workspace-clone`).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ExistingAgentsMd => "existing-agents-md",
            Self::WorkspaceClone => "workspace-clone",
        }
    }
}

/// Generate root `AGENTS.md` and `.emery/context.lock` for a freshly
/// initialised project.
///
/// Returns `Ok(None)` when generation ran, `Ok(Some(reason))` when it
/// was skipped. Reads the just-written `project.yaml`, so every init
/// branch shares one call after scaffold completes.
///
/// # Errors
///
/// Bubbles up filesystem, config-load, and adapter-resolution errors.
pub fn generate(resolver: &impl Resolver, paths: &ExecutionPaths) -> Result<Option<Skip>, Error> {
    let project_dir = paths.project_root();
    if is_slot(project_dir) {
        return Ok(Some(Skip::WorkspaceClone));
    }
    match project_dir.join("AGENTS.md").try_exists() {
        Ok(true) => return Ok(Some(Skip::ExistingAgentsMd)),
        Ok(false) => {}
        Err(err) if err.kind() == ErrorKind::NotFound => {}
        Err(err) => return Err(Error::Io(err)),
    }

    let config = ProjectConfig::load(project_dir)?;
    let adapter = resolve_adapter_name(resolver, paths, &config)?;
    let version = env!("CARGO_PKG_VERSION");
    let body = render_body(adapter.as_deref());
    let body_digest = format!("sha256:{}", diagnostics::digest::sha256_hex(body.as_bytes()));
    let document = format!(
        "# {name} - Agent Instructions\n\n\
         <!-- emery:context begin\n\
         fingerprint: {body_digest}\n\
         generated-by: emery {version}\n\
         -->\n\n\
         {body}\
         <!-- emery:context end -->\n",
        name = one_line(&config.name),
    );

    bytes_write(&project_dir.join("AGENTS.md"), document.as_bytes())?;
    yaml_write(
        &Layout::new(project_dir).emery_dir().join("context.lock"),
        &ContextLock {
            version: 1,
            fingerprint: body_digest.clone(),
            cli_version: version.to_string(),
            body_sha256: body_digest,
        },
    )?;
    Ok(None)
}

#[derive(Debug, Serialize)]
struct ContextLock {
    version: u64,
    fingerprint: String,
    cli_version: String,
    body_sha256: String,
}

fn resolve_adapter_name(
    resolver: &impl Resolver, paths: &ExecutionPaths, config: &ProjectConfig,
) -> Result<Option<String>, Error> {
    let Some(value) = config.adapter.as_deref().filter(|_| !config.workspace) else {
        return Ok(None);
    };
    let selector = AdapterSelector::parse(value)?;
    let target = resolver.resolve_target(&selector, paths)?;
    Ok(Some(target.manifest.name))
}

fn render_body(adapter: Option<&str>) -> String {
    let adapter_line = adapter.map_or_else(
        || "- no bound target adapter yet.\n".to_string(),
        |name| format!("- adapter `{name}`.\n"),
    );
    format!(
        "## Conventions\n\
         - Prefer `emery` CLI verbs over hand-editing `.emery/` state.\n\
         {adapter_line}\
         \n## Boundaries\n\
         - During execute/build/merge, agents consume Emery and adapters — they do not maintain them.\n\
         - `metadata.yaml` and `plan.yaml` are framework-managed; update them through `emery` verbs.\n\
         - `.emery/archive/` is framework-managed history.\n\n",
    )
}

fn one_line(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

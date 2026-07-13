//! Init-time `AGENTS.md` context generation.
//!
//! Assembles a [`render::Input`] from the freshly scaffolded project
//! (config, registry, adapter, root markers, active slices), renders
//! the fenced document, and persists `AGENTS.md` plus the
//! `.specify/context.lock` fingerprint sidecar. Generation is
//! skipped — never overwriting — when `AGENTS.md` already exists or
//! the project is a materialised workspace slot.

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use artifacts::atomic::bytes_write;
use error::Error;

use crate::adapter::{AdapterRef, ResolvedTarget, Resolver};
use crate::agents::{detect, fences, fingerprint, lock, render};
use crate::config::{Layout, ProjectConfig, is_slot};
use crate::registry::{Registry, RegistryProject, TopologyLock, TopologyProject};
use crate::slice::SliceMetadata;

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

/// Generate root `AGENTS.md` and `.specify/context.lock` for a freshly
/// initialised project.
///
/// Returns `Ok(None)` when generation ran, `Ok(Some(reason))` when it
/// was skipped (existing `AGENTS.md`, or init inside a materialised
/// workspace slot). Reads the just-written `project.yaml`, so every
/// init branch (regular, workspace, upgrade) shares one call after its
/// scaffold completes — the skip logic gives `--upgrade` its
/// "regenerate only when absent" behavior for free.
///
/// # Errors
///
/// Bubbles up filesystem, config-load, adapter-resolution, and
/// fence-composition errors.
pub(super) fn generate(
    resolver: &impl Resolver, project_dir: &Path,
) -> Result<Option<Skip>, Error> {
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
    let assembly = render_input(resolver, project_dir, &config)?;
    let aggregate = fingerprint::aggregate(env!("CARGO_PKG_VERSION"), assembly.inputs.clone());
    let generated = render::render_document(&assembly.input, &aggregate);
    let fenced = fences::parse_document(generated.as_bytes())
        .map_err(|err| Error::Diag {
            code: "context-generated-document-fence-error",
            detail: err.to_string(),
        })?
        .ok_or_else(|| Error::Diag {
            code: "context-generated-document-missing-fences",
            detail: "generated AGENTS.md content must contain a Specify context fence".to_string(),
        })?;
    let context_fingerprint =
        fingerprint::for_context(env!("CARGO_PKG_VERSION"), assembly.inputs, fenced.body());

    bytes_write(&project_dir.join("AGENTS.md"), generated.as_bytes())?;
    lock::save(
        &Layout::new(project_dir).specify_dir().join("context.lock"),
        &lock::ContextLock::from_fingerprint(&context_fingerprint),
    )?;
    Ok(None)
}

struct Assembly {
    input: render::Input,
    inputs: Vec<fingerprint::InputFingerprint>,
}

/// Walk the project (adapter, registry, slices, root markers) into a
/// [`render::Input`] plus the per-input fingerprint set.
fn render_input(
    resolver: &impl Resolver, project_dir: &Path, config: &ProjectConfig,
) -> Result<Assembly, Error> {
    let layout = Layout::new(project_dir);
    let mut collector = fingerprint::InputCollector::new(project_dir);
    collector.add_file(&layout.config_path())?;

    let registry = Registry::load(project_dir)?;
    collector.add_file_if_present(&layout.registry_path())?;

    let adapter = match config.adapter.as_deref().filter(|_| !config.workspace) {
        Some(value) => {
            let target_ref = AdapterRef::from_value(value);
            let target = resolver.resolve_target(&target_ref, project_dir)?;
            collect_adapter_input(&mut collector, &target_ref, &target);
            Some(render::Adapter {
                name: target.manifest.name,
                version: target.manifest.version,
            })
        }
        None => None,
    };
    // Detection markers only make sense for a regular project; a
    // registry-only workspace renders no per-language sections.
    let detection = if config.workspace {
        detect::Detection::default()
    } else {
        detect::detect_root_markers(project_dir)
    };
    collector.add_relative_files(detection.input_paths.iter().map(String::as_str))?;

    let active_slices = slice_names(&layout.slices_dir(), &mut collector)?;

    let input = render::Input {
        project_name: config.name.clone(),
        is_workspace: config.workspace,
        detection,
        description: config.description.clone(),
        adapter,
        rule_overrides: rule_overrides(config),
        active_slices,
        workspace_peers: workspace_peers(registry.as_ref(), project_dir)?,
        dependencies: dependency_peers(registry.as_ref(), project_dir),
    };
    Ok(Assembly {
        input,
        inputs: collector.finalize()?,
    })
}

/// Record the adapter component bytes under a machine-independent
/// logical id. The resolved component lives at a machine-specific
/// absolute path (the global store or a dev build tree), so the
/// fingerprint keys on `<origin-label>:<identity>` while the physical
/// bytes still drive the digest. A non-file origin (the native test
/// resolver) contributes no input.
fn collect_adapter_input(
    collector: &mut fingerprint::InputCollector, adapter_ref: &AdapterRef,
    resolved: &ResolvedTarget,
) {
    let path = Path::new(&resolved.origin.reference);
    if !path.is_file() {
        return;
    }
    let identity = adapter_ref.version.as_ref().map_or_else(
        || format!("{}.wasm", adapter_ref.name),
        |version| format!("{}@{version}.wasm", adapter_ref.name),
    );
    collector.add_file_as(&format!("{}:{identity}", resolved.origin.label), path);
}

fn rule_overrides(config: &ProjectConfig) -> Vec<render::Rule> {
    let mut overrides: Vec<render::Rule> = config
        .rules
        .iter()
        .filter(|(_brief_id, path)| !path.is_empty())
        .map(|(brief_id, path)| render::Rule {
            brief_id: brief_id.clone(),
            path: format!(".specify/{path}"),
        })
        .collect();
    overrides
        .sort_by(|left, right| (&left.brief_id, &left.path).cmp(&(&right.brief_id, &right.path)));
    overrides
}

fn dependency_peers(registry: Option<&Registry>, project_dir: &Path) -> Vec<render::Dep> {
    let Some(registry) = registry else {
        return Vec::new();
    };
    if registry.projects.len() <= 1 {
        return Vec::new();
    }

    // Peer adapter/description come from the committed
    // `.specify/topology.lock` (each member project's authored
    // `project.yaml`). A fresh workspace may not have synced a cache
    // yet, so fall back to the registry's optional greenfield seed.
    let lock = TopologyLock::load(&Layout::new(project_dir).topology_lock_path()).ok().flatten();
    let facets: HashMap<&str, &TopologyProject> = lock
        .as_ref()
        .map(|lock| lock.projects.iter().map(|p| (p.name.as_str(), p)).collect())
        .unwrap_or_default();

    let mut peers: Vec<render::Dep> = registry
        .projects
        .iter()
        .map(|project| {
            let cached = facets.get(project.name.as_str());
            render::Dep {
                name: project.name.clone(),
                adapter: cached
                    .map(|c| c.target.clone())
                    .or_else(|| project.adapter.clone())
                    .unwrap_or_default(),
                url: project.url.clone(),
                description: cached
                    .and_then(|c| c.description.clone())
                    .or_else(|| project.description.clone()),
            }
        })
        .collect();
    peers.sort_by(|left, right| {
        (&left.name, &left.adapter, &left.url).cmp(&(&right.name, &right.adapter, &right.url))
    });
    peers
}

fn workspace_peers(
    registry: Option<&Registry>, project_dir: &Path,
) -> Result<Vec<render::Peer>, Error> {
    let Some(registry) = registry else {
        return Ok(Vec::new());
    };
    if registry.projects.len() <= 1 {
        return Ok(Vec::new());
    }

    let workspace_dir = project_dir.join("workspace");
    let mut peers = Vec::new();
    for RegistryProject { name, .. } in &registry.projects {
        match fs::symlink_metadata(workspace_dir.join(name)) {
            Ok(_) => peers.push(render::Peer {
                name: name.clone(),
                path: format!("workspace/{name}/"),
            }),
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(Error::Io(err)),
        }
    }
    peers.sort_by(|left, right| (&left.path, &left.name).cmp(&(&right.path, &right.name)));
    Ok(peers)
}

fn slice_names(
    slices_dir: &Path, collector: &mut fingerprint::InputCollector,
) -> Result<Vec<String>, Error> {
    if !slices_dir.exists() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    for entry in fs::read_dir(slices_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let metadata_path = SliceMetadata::path(&entry.path());
        if !metadata_path.is_file() {
            continue;
        }
        collector.add_file(&metadata_path)?;
        if let Some(name) = entry.file_name().to_str() {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names)
}

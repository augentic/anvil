//! Render-input assembly for init-time AGENTS.md generation. Walks the project
//! (adapter, registry, slices, root markers) and emits a
//! [`render::Input`] plus the per-input fingerprint set.

use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use specify_error::{Error, Result};
use specify_workflow::adapter::{ADAPTER_FILENAME, AdapterLocation, ResolvedTargetAdapter};
use specify_workflow::agents::{detect, fingerprint, render};
use specify_workflow::config::{Layout, ProjectConfig};
use specify_workflow::registry::{Registry, TopologyLock, TopologyProject};
use specify_workflow::slice::SliceMetadata;

use crate::runtime::context::Ctx;

pub(super) struct RenderAssembly {
    pub(super) input: render::Input,
    pub(super) inputs: Vec<fingerprint::InputFingerprint>,
}

pub(super) fn render_input(ctx: &Ctx) -> Result<RenderAssembly> {
    let layout = ctx.layout();
    let mut collector = fingerprint::InputCollector::new(&ctx.project_dir);
    collector.add_file(&layout.config_path())?;

    let registry = Registry::load(&ctx.project_dir)?;
    collector.add_file_if_present(&layout.registry_path())?;

    let adapter = if ctx.config.workspace {
        None
    } else {
        let resolved = ctx.resolve_target_adapter()?;
        collect_adapter_inputs(&mut collector, &resolved, &ctx.project_dir)?;
        Some(adapter_summary(&resolved))
    };
    let detection = if ctx.config.workspace {
        detect::Detection::default()
    } else {
        detect::detect_root_markers(&ctx.project_dir)
    };
    emit_detection_warnings(&detection.warnings);
    collector.add_relative_files(detection.input_paths.iter().map(String::as_str))?;

    // The renderer's `plan.yaml` guidance is unconditional navigation text. It
    // does not inspect `plan.yaml` existence or content, so that file is not
    // fingerprinted unless a future renderer contract actually reads it.
    let active_slices = active_slice_names(&layout.slices_dir(), &mut collector)?;

    let input = render::Input {
        project_name: ctx.config.name.clone(),
        is_workspace: ctx.config.workspace,
        detection,
        description: ctx.config.description.clone(),
        adapter,
        rule_overrides: rule_overrides(&ctx.config),
        declared_tools: declared_tools(&ctx.config),
        active_slices,
        workspace_peers: materialized_workspace_peers(registry.as_ref(), &ctx.project_dir)?,
        dependencies: dependency_peers(registry.as_ref(), &ctx.project_dir),
    };
    Ok(RenderAssembly {
        input,
        inputs: collector.finalize()?,
    })
}

fn collect_adapter_inputs(
    collector: &mut fingerprint::InputCollector, adapter: &ResolvedTargetAdapter,
    project_dir: &Path,
) -> Result<()> {
    let manifest = adapter.location.path().join(ADAPTER_FILENAME);
    add_adapter_input(collector, project_dir, &adapter.location, &manifest)?;
    for relative in adapter.manifest.briefs.values() {
        let brief_path = adapter.location.path().join(relative);
        add_adapter_input(collector, project_dir, &adapter.location, &brief_path)?;
    }
    Ok(())
}

/// Record one adapter input file. A project-local adapter is recorded
/// by its in-tree path; a cache-resolved adapter lives out-of-tree, so
/// it is recorded under a cache-relative `cache:...` token instead (the
/// physical bytes still drive the digest).
fn add_adapter_input(
    collector: &mut fingerprint::InputCollector, project_dir: &Path, location: &AdapterLocation,
    path: &Path,
) -> Result<()> {
    if !path.is_file() {
        return Ok(());
    }
    match location {
        AdapterLocation::Local(_) => collector.add_file(path)?,
        AdapterLocation::Cached(_) => {
            let cache_dir = Layout::new(project_dir).cache_dir();
            collector.add_file_as(&logical_path("cache", &cache_dir, path), path);
        }
        AdapterLocation::Store(_) => {
            // The global store root is machine-specific; key the
            // fingerprint on the stable `store:<name>@<version>/<rel>`
            // logical path so the context lock is reproducible.
            let store_root = specify_schema::cache::adapter_store_root();
            collector.add_file_as(&logical_path("store", &store_root, path), path);
        }
    }
    Ok(())
}

/// Machine-independent logical id for a file resolved beneath an
/// out-of-tree `root` (the per-project manifest cache or the global
/// adapter store). Strips `root` and joins the relative tail with `/`
/// under a `<prefix>:` scheme so context-lock fingerprints stay stable
/// across machines; falls back to the absolute path when `path` is not
/// under `root`.
fn logical_path(prefix: &str, root: &Path, path: &Path) -> String {
    path.strip_prefix(root).map_or_else(
        |_| format!("{prefix}:{}", path.display()),
        |rel| {
            let rel = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy())
                .collect::<Vec<_>>()
                .join("/");
            format!("{prefix}:{rel}")
        },
    )
}

fn adapter_summary(adapter: &ResolvedTargetAdapter) -> render::Adapter {
    let mut briefs: Vec<render::Brief> = adapter
        .manifest
        .briefs
        .keys()
        .map(|operation| {
            let label = operation.to_string();
            render::Brief {
                phase: label.clone(),
                id: label,
                description: String::new(),
            }
        })
        .collect();
    briefs.sort_by(|left, right| {
        (&left.phase, &left.id, &left.description).cmp(&(
            &right.phase,
            &right.id,
            &right.description,
        ))
    });

    render::Adapter {
        name: adapter.manifest.name.clone(),
        version: adapter.manifest.version.clone(),
        description: adapter.manifest.description.clone().unwrap_or_default(),
        briefs,
    }
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

fn declared_tools(config: &ProjectConfig) -> Vec<render::Extension> {
    let mut tools: Vec<render::Extension> = config
        .tools
        .iter()
        .map(|tool| render::Extension {
            name: tool.name.clone(),
            version: tool.version.clone(),
        })
        .collect();
    tools.sort_by(|left, right| (&left.name, &left.version).cmp(&(&right.name, &right.version)));
    tools
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
    // `project.yaml`). A fresh workspace may not have synced a cache yet, so
    // fall back to the registry's optional greenfield seed.
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

fn materialized_workspace_peers(
    registry: Option<&Registry>, project_dir: &Path,
) -> Result<Vec<render::Peer>> {
    let Some(registry) = registry else {
        return Ok(Vec::new());
    };
    if registry.projects.len() <= 1 {
        return Ok(Vec::new());
    }

    let workspace_dir = project_dir.join("workspace");
    let mut peers = Vec::new();
    for project in &registry.projects {
        let path = workspace_dir.join(&project.name);
        match fs::symlink_metadata(path) {
            Ok(_) => peers.push(render::Peer {
                name: project.name.clone(),
                path: format!("workspace/{}/", project.name),
            }),
            Err(err) if err.kind() == ErrorKind::NotFound => {}
            Err(err) => return Err(Error::Io(err)),
        }
    }
    peers.sort_by(|left, right| (&left.path, &left.name).cmp(&(&right.path, &right.name)));
    Ok(peers)
}

fn active_slice_names(
    slices_dir: &Path, collector: &mut fingerprint::InputCollector,
) -> Result<Vec<String>> {
    if !slices_dir.exists() {
        return Ok(Vec::new());
    }

    let mut names = Vec::new();
    for entry in fs::read_dir(slices_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let candidate = entry.path();
        let metadata_path = SliceMetadata::path(&candidate);
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

fn emit_detection_warnings(warnings: &[detect::DetectionWarning]) {
    for warning in warnings {
        eprintln!("warning: {}: {}", warning.path, warning.message);
    }
}

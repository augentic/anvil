//! Re-entry (`emery init --upgrade`) body: bumps `project.yaml.emery`
//! to the running binary over an existing `.emery/` without re-scaffolding.
//! Mutates only `project.yaml`; never touches slices, specs, archive, registry,
//! or the adapter cache.
//!
//! One runner serves both regular and workspace projects: the
//! preservation logic is identical, so the dispatcher routes here ahead
//! of the workspace / regular branch. The recorded adapter binding was
//! already re-ensured (and resolved) by the operation layer; the
//! selector itself is never rewritten.

use std::fs;

use error::Error;

use crate::adapter::ComponentMeta;
use crate::config::{Layout, ProjectConfig};
use crate::init::{InitOptions, InitResult, resolve_version, validate_platforms};

/// Run the re-entry version bump.
///
/// Loads the existing config, then bumps the `emery` pin to the
/// running binary's version — but only when it differs, so an
/// already-current project is a true no-op (no `project.yaml` write).
///
/// # Errors
///
/// - [`Error::NotInitialized`] when `.emery/project.yaml` is absent —
///   `--upgrade` requires an existing project.
/// - [`Error::CliTooOld`] when the pinned floor is newer than this
///   binary (propagated by the loader).
/// - filesystem / serialisation errors from rewriting `project.yaml`.
pub(super) fn run(opts: InitOptions<'_>) -> Result<InitResult, Error> {
    let mut cfg = ProjectConfig::load(opts.project_dir)?;

    let layout = Layout::new(opts.project_dir);
    let config_path = layout.config_path();
    let target = resolve_version();

    let platforms_changed = if let Some(incoming) = opts.platforms {
        let ensured = opts.adapter.ok_or_else(|| Error::Diag {
            code: "upgrade-platforms-no-adapter",
            detail:
                "--platforms requires a project with a bound target adapter (workspace projects \
                     have no adapter)"
                    .to_string(),
        })?;
        let validated = validate_platforms(
            Some(incoming),
            ensured.resolved.manifest.platforms.as_ref(),
            &ensured.resolved.manifest.name,
        )?;
        let changed = cfg.platforms != validated;
        cfg.platforms = validated;
        changed
    } else {
        false
    };

    let emery_version_changed = cfg.emery_version.as_deref() != Some(target.as_str());
    let needs_write = emery_version_changed || platforms_changed;
    if emery_version_changed {
        cfg.emery_version = Some(target.clone());
    }
    if needs_write {
        let serialised = serde_saphyr::to_string(&cfg)?;
        fs::write(&config_path, serialised)?;
    }

    let adapter_name = if cfg.workspace {
        "workspace".to_string()
    } else {
        opts.adapter.map_or_else(String::new, |ensured| ensured.resolved.manifest.name.clone())
    };

    Ok(InitResult {
        config_path,
        cache_present: !cfg.workspace && ComponentMeta::path(opts.paths, &adapter_name).exists(),
        adapter_name,
        directories_created: Vec::new(),
        scaffolded_rule_keys: Vec::new(),
        emery_version: target,
        context_skip_reason: None,
    })
}

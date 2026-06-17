//! Re-entry (`specify init --upgrade`) body: bumps `project.yaml.specify_version`
//! to the running binary over an existing `.specify/` without re-scaffolding.
//! Mutates only `project.yaml`; never touches slices, specs, archive, registry,
//! or the adapter cache.
//!
//! One runner serves both regular and workspace projects: the
//! preservation logic is identical, so the dispatcher routes here ahead
//! of the workspace / regular branch.

use std::fs;

use specify_error::Error;

use crate::adapter::TargetAdapter;
use crate::config::{Layout, ProjectConfig};
use crate::init::adapter_uri::{adapter_name_from_value, adapter_ref_from_value};
use crate::init::cache::{CodexMeta, ManifestMeta};
use crate::init::{InitOptions, InitResult, resolve_version, validate_platforms};

/// Run the re-entry version bump.
///
/// Loads the existing config, then bumps `specify_version` to the
/// running binary's version — but only when it differs, so an
/// already-current project is a true no-op (no `project.yaml` write).
///
/// # Errors
///
/// - [`Error::NotInitialized`] when `.specify/project.yaml` is absent —
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
        let adapter_value = cfg.adapter.as_deref().ok_or_else(|| Error::Diag {
            code: "upgrade-platforms-no-adapter",
            detail:
                "--platforms requires a project with a bound target adapter (workspace projects \
                     have no adapter)"
                    .to_string(),
        })?;
        let adapter_ref = adapter_ref_from_value(adapter_value);
        let resolved = TargetAdapter::resolve(&adapter_ref, opts.project_dir)?;
        let validated = validate_platforms(
            Some(incoming),
            resolved.manifest.platforms.as_ref(),
            &adapter_ref.name,
        )?;
        let changed = cfg.platforms != validated;
        cfg.platforms = validated;
        changed
    } else {
        false
    };

    let specify_version_changed = cfg.specify_version.as_deref() != Some(target.as_str());
    let needs_write = specify_version_changed || platforms_changed;
    if specify_version_changed {
        cfg.specify_version = Some(target.clone());
    }
    if needs_write {
        let serialised = serde_saphyr::to_string(&cfg)?;
        fs::write(&config_path, serialised)?;
    }

    let adapter_name = if cfg.workspace {
        "workspace".to_string()
    } else {
        cfg.adapter
            .as_deref()
            .map_or_else(String::new, |value| adapter_name_from_value(value).to_string())
    };

    Ok(InitResult {
        config_path,
        adapter_name,
        cache_present: ManifestMeta::path(opts.project_dir).exists(),
        codex_present: CodexMeta::path(opts.project_dir).exists(),
        directories_created: Vec::new(),
        scaffolded_rule_keys: Vec::new(),
        specify_version: target,
        specify_version_changed,
        wasm_pkg_config_written: false,
    })
}

#[cfg(test)]
mod tests;

//! Re-entry (`emery init --upgrade`): bumps `project.yaml.emery` over
//! an existing `.emery/` without re-scaffolding. Mutates only
//! `project.yaml`; the recorded adapter binding is never rewritten.

use error::Error;

use crate::adapter::ComponentMeta;
use crate::config::{Layout, ProjectConfig};
use crate::init::{InitOptions, InitResult, binding_value, resolve_version, validate_platforms};

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
            detail: "--platforms requires a project with a bound target adapter".to_string(),
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

    // Preserved when `--gap-policy` is absent (the load-then-rewrite
    // carries the recorded value); updated when passed, mirroring
    // `--platforms`.
    let gap_policy_changed = match opts.gap_policy {
        Some(policy) if cfg.gap_policy != Some(policy) => {
            cfg.gap_policy = Some(policy);
            true
        }
        _ => false,
    };

    // The recorded binding is reported, never rewritten: a bare
    // record stays bare under local-first resolution.
    let adapter_binding = match opts.adapter {
        Some(ensured) => Some(binding_value(ensured, opts.paths, opts.project_dir)?),
        None => None,
    };

    let emery_version_changed = cfg.emery_version.as_deref() != Some(target.as_str());
    let needs_write = emery_version_changed || platforms_changed || gap_policy_changed;
    if emery_version_changed {
        cfg.emery_version = Some(target.clone());
    }
    if needs_write {
        artifacts::atomic::yaml_write(&config_path, &cfg)?;
    }

    let adapter_name =
        opts.adapter.map_or_else(String::new, |ensured| ensured.resolved.manifest.name.clone());

    Ok(InitResult {
        config_path,
        cache_present: ComponentMeta::path(opts.paths, &adapter_name).exists(),
        adapter_name,
        adapter_binding,
        directories_created: Vec::new(),
        scaffolded_rule_keys: Vec::new(),
        emery_version: target,
        context_skip_reason: None,
    })
}

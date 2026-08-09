//! Fresh init body. Scaffolds the per-project `.emery/` tree over the
//! ensured adapter binding and writes `project.yaml`.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use error::Error;

use crate::adapter::ComponentMeta;
use crate::config::{Layout, ProjectConfig};
use crate::init::{
    InitOptions, InitResult, binding_value, resolve_version, resolved_name, upsert_gitignore,
    validate_platforms,
};

/// canonical refine-time artifact set. Hardcoded — refine synthesises
/// the canonical set directly rather than reading it from the target
/// adapter. The
/// exact scaffold keys mirror the validation registry namespaces in
/// `artifacts::validate::registry::rules_for`.
const SCAFFOLDED_RULE_KEYS: &[&str] = &["proposal", "specs", "design", "tasks"];

pub(super) fn run(opts: InitOptions<'_>) -> Result<InitResult, Error> {
    let ensured = opts.adapter.ok_or_else(|| Error::Diag {
        code: "init-adapter-required",
        detail: "pass <adapter>".to_string(),
    })?;
    let name = resolved_name(opts.project_dir, opts.name);
    let layout = Layout::new(opts.project_dir);

    let mut directories_created: Vec<PathBuf> = Vec::new();
    // Repo-root artefacts (`change.md`, `plan.yaml`) and the
    // out-of-tree memoization cache are minted on demand by their
    // owners; `.emery/specs/` is a convention the omnia adapter uses.
    for dir in [
        layout.emery_dir(),
        layout.slices_dir(),
        layout.emery_dir().join("specs"),
        layout.archive_dir(),
    ] {
        let already = dir.exists();
        fs::create_dir_all(&dir)?;
        if !already {
            directories_created.push(dir);
        }
    }

    // Persist the selector as typed: a bare name stays bare — the
    // deployment resolves it local-first at every use.
    let adapter_name = ensured.resolved.manifest.name.clone();
    let adapter_value = binding_value(ensured, opts.paths, opts.project_dir)?;
    let validated_platforms = validate_platforms(
        opts.platforms,
        ensured.resolved.manifest.platforms.as_ref(),
        &adapter_name,
    )?;
    let scaffolded_rule_keys: Vec<String> =
        SCAFFOLDED_RULE_KEYS.iter().map(|key| (*key).to_string()).collect();

    let emery_version = resolve_version();

    let mut rules: BTreeMap<String, String> = BTreeMap::new();
    for key in &scaffolded_rule_keys {
        rules.insert(key.clone(), String::new());
    }
    let cfg = ProjectConfig {
        name,
        description: opts.description.map(str::to_string),
        adapter: Some(adapter_value.clone()),
        emery_version: Some(emery_version.clone()),
        rules,
        platforms: validated_platforms,
    };

    let config_path = layout.config_path();
    artifacts::atomic::yaml_write(&config_path, &cfg)?;

    upsert_gitignore(opts.project_dir)?;

    let cache_present = ComponentMeta::path(opts.paths, &adapter_name).exists();

    Ok(InitResult {
        config_path,
        adapter_name,
        adapter_binding: Some(adapter_value),
        cache_present,
        directories_created,
        scaffolded_rule_keys,
        emery_version,
        context_skip_reason: None,
    })
}

//! Regular (non-workspace) init body. Scaffolds the per-project `.specify/`
//! tree, resolves the requested adapter into the out-of-tree cache, and
//! writes `project.yaml`.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use jiff::Timestamp;
use specify_error::Error;

use crate::adapter::TargetAdapter;
use crate::config::{Layout, ProjectConfig};
use crate::init::adapter_uri::adapter_ref_from_value;
use crate::init::cache::{ManifestMeta, cache_adapter, cache_codex};
use crate::init::{
    InitOptions, InitResult, resolve_version, resolved_name, scaffold_wasm_pkg_config,
    upsert_gitignore, validate_platforms,
};

/// canonical refine-time artifact set. Hardcoded because target
/// adapters no longer enumerate per-define-brief artifacts via
/// `pipeline.define[]`; refine synthesises the canonical set directly
/// (see `DECISIONS.md` §"Adapter loader axis routing"). The exact
/// scaffold keys mirror the validation registry namespaces in
/// `specify_model::validate::registry::rules_for`.
const SCAFFOLDED_RULE_KEYS: &[&str] = &["proposal", "specs", "design", "tasks"];

pub(super) fn run(opts: InitOptions<'_>, now: Timestamp) -> Result<InitResult, Error> {
    let adapter = opts.adapter.ok_or_else(|| Error::Diag {
        code: "init-requires-adapter-or-workspace",
        detail: "pass <adapter> or --workspace".to_string(),
    })?;
    let name = resolved_name(opts.project_dir, opts.name);
    let layout = Layout::new(opts.project_dir);

    let mut directories_created: Vec<PathBuf> = Vec::new();
    // Repo-root artefacts (`registry.yaml`, `change.md`, `plan.yaml`)
    // are not pre-touched — their owning verbs mint them on demand.
    // `.specify/specs/` is retained as a per-project convention used
    // by the bundled `omnia` adapter.
    // The memoization cache is out-of-tree (OS cache, created on demand
    // by `cache_adapter` / `cache_codex`), so it is not scaffolded here.
    for dir in [
        layout.specify_dir(),
        layout.slices_dir(),
        layout.specify_dir().join("specs"),
        layout.archive_dir(),
    ] {
        let already = dir.exists();
        fs::create_dir_all(&dir)?;
        if !already {
            directories_created.push(dir);
        }
    }

    let source = cache_adapter(adapter, opts.project_dir, now)?;
    // Distribute the shared codex from the same resolved checkout
    // (pinned to the adapter source/ref) before the checkout guard in
    // `source` drops. Fail-soft: a source tree without the shared pack
    // leaves `codex_present` false.
    let codex_present = cache_codex(opts.project_dir, &source, opts.include_framework, now)?;
    let adapter_value = source.adapter_value;
    let adapter_ref = adapter_ref_from_value(&adapter_value);
    let resolved = TargetAdapter::resolve(&adapter_ref, opts.project_dir)?;
    let adapter_name = resolved.manifest.name.clone();
    let validated_platforms =
        validate_platforms(opts.platforms, resolved.manifest.platforms.as_ref(), &adapter_name)?;
    let scaffolded_rule_keys: Vec<String> =
        SCAFFOLDED_RULE_KEYS.iter().map(|key| (*key).to_string()).collect();

    let specify_version = resolve_version();

    let mut rules: BTreeMap<String, String> = BTreeMap::new();
    for key in &scaffolded_rule_keys {
        rules.insert(key.clone(), String::new());
    }
    let cfg = ProjectConfig {
        name,
        description: opts.description.map(str::to_string),
        adapter: Some(adapter_value),
        specify_version: Some(specify_version.clone()),
        rules,
        tools: Vec::new(),
        platforms: validated_platforms,
        workspace: false,
    };

    let config_path = layout.config_path();
    let serialised = serde_saphyr::to_string(&cfg)?;
    fs::write(&config_path, serialised)?;

    let wasm_pkg_config_written = scaffold_wasm_pkg_config(&layout)?;

    upsert_gitignore(opts.project_dir)?;

    let cache_present = ManifestMeta::path(opts.project_dir).exists();

    Ok(InitResult {
        config_path,
        adapter_name,
        cache_present,
        codex_present,
        directories_created,
        scaffolded_rule_keys,
        specify_version,
        specify_version_changed: true,
        wasm_pkg_config_written,
    })
}

#[cfg(test)]
mod tests;

//! Regular (non-workspace) init body. Scaffolds the per-project `.specify/`
//! tree over the ensured adapter binding and writes `project.yaml`.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use error::Error;

use crate::adapter::{AdapterSelector, ComponentMeta};
use crate::config::{Layout, ProjectConfig};
use crate::init::{
    InitOptions, InitResult, resolve_version, resolved_name, upsert_gitignore, validate_platforms,
};

/// canonical refine-time artifact set. Hardcoded — refine synthesises
/// the canonical set directly rather than reading it from the target
/// adapter. The
/// exact scaffold keys mirror the validation registry namespaces in
/// `artifacts::validate::registry::rules_for`.
const SCAFFOLDED_RULE_KEYS: &[&str] = &["proposal", "specs", "design", "tasks"];

pub(super) fn run(opts: InitOptions<'_>) -> Result<InitResult, Error> {
    let ensured = opts.adapter.ok_or_else(|| Error::Diag {
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
    // by the provider's ensure), so it is not scaffolded here.
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

    // Persist the operator's selector as typed (the kind is never
    // rewritten). A component path is recorded as its canonical
    // `file://` form so the value outlives the CWD — read from the
    // cache mirror's provenance sidecar when present, because the
    // engine guest cannot canonicalize a host path that lives outside
    // its mounts (the launcher mirrored and stamped it host-side
    // before the runtime started); canonicalized directly otherwise.
    let adapter_name = ensured.resolved.manifest.name.clone();
    let adapter_value = match &ensured.selector {
        AdapterSelector::Component { .. } => ComponentMeta::load(opts.paths, &adapter_name)
            .map_or_else(
                || ensured.selector.persist_value(opts.project_dir),
                |meta| Ok(meta.source),
            )?,
        _ => ensured.selector.persist_value(opts.project_dir)?,
    };
    let validated_platforms = validate_platforms(
        opts.platforms,
        ensured.resolved.manifest.platforms.as_ref(),
        &adapter_name,
    )?;
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
        platforms: validated_platforms,
        workspace: false,
    };

    let config_path = layout.config_path();
    let serialised = serde_saphyr::to_string(&cfg)?;
    fs::write(&config_path, serialised)?;

    upsert_gitignore(opts.project_dir)?;

    let cache_present = ComponentMeta::path(opts.paths, &adapter_name).exists();

    Ok(InitResult {
        config_path,
        adapter_name,
        cache_present,
        directories_created,
        scaffolded_rule_keys,
        specify_version,
        context_skip_reason: None,
    })
}

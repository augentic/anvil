//! Per-run source bindings from argv or an operator-owned `emery.toml`.
//!
//! The binding list is an input, never engine state: nothing here is
//! persisted, and the engine never writes the config file. A run that
//! names no bindings at all discovers the project-root `emery.toml`
//! as a fallback — never merged with argv bindings.

use std::path::{Path, PathBuf};

use omnia_guest::plugins::Digest;
use omnia_guest::{Error, bad_request, server_error};

use crate::handler::preopen_path;
use crate::resolve::AdapterSelector;

/// The project-root config discovered by a bindingless run.
pub const CONFIG_FILE: &str = "emery.toml";

/// A source binding for one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBinding {
    /// Stable binding key.
    pub key: String,
    /// The adapter selector value.
    pub adapter: String,
    /// What the adapter extracts.
    pub content: BindingContent,
    /// Optional sha256 content pin for a loader-loaded adapter,
    /// verified host-side before validation.
    pub digest: Option<Digest>,
    /// Optional registry endpoint override for a package adapter;
    /// `None` selects the acquirer's default.
    pub registry: Option<String>,
}

/// Workspace or inline source content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingContent {
    /// Project-relative read-only root; `.` binds the project.
    Workspace(String),
    /// Inline description text; no filesystem view.
    Description(String),
}

/// Builds the run's binding list from `emery specify` inputs.
///
/// # Errors
///
/// Returns a `BadRequest` when `--config` is mixed with positional
/// adapters or `--description` bindings, and propagates the argv,
/// file, and discovery loader failures.
pub fn bindings(
    adapters: &[String], descriptions: &[String], config: Option<&str>,
) -> Result<Vec<SourceBinding>, Error> {
    match config {
        Some(path) => {
            if !adapters.is_empty() || !descriptions.is_empty() {
                return Err(bad_request!(
                    "cannot be combined with positional `<adapter>` or `--description` bindings; \
                     the file carries the whole binding list"
                ));
            }
            from_file(&preopen_path(Path::new(path), "--config")?)
        }
        None if adapters.is_empty() && descriptions.is_empty() => discover(),
        None => from_argv(adapters, descriptions),
    }
}

// The discovery fallback: a bindingless run reads the project-root
// `emery.toml` when present; a parse failure still refuses typed.
fn discover() -> Result<Vec<SourceBinding>, Error> {
    let path = Path::new(CONFIG_FILE);
    let present = path.try_exists().map_err(|source| server_error!("{CONFIG_FILE} ({source})",))?;
    if present { from_file(path) } else { Err(source_required()) }
}

// Argv bindings: each positional adapter lends the workspace at `.`;
// each `--description` entry is inline. The key is the adapter name.
fn from_argv(adapters: &[String], descriptions: &[String]) -> Result<Vec<SourceBinding>, Error> {
    let mut bindings = Vec::new();
    for value in adapters {
        let key = AdapterSelector::parse(value)?.name()?;
        push_unique(
            &mut bindings,
            SourceBinding {
                key,
                adapter: value.clone(),
                content: BindingContent::Workspace(".".to_string()),
                digest: None,
                registry: None,
            },
        )?;
    }
    for entry in descriptions {
        let (adapter, text) = split_description(entry)?;
        let key = AdapterSelector::parse(adapter)?.name()?;
        push_unique(
            &mut bindings,
            SourceBinding {
                key,
                adapter: adapter.to_string(),
                content: BindingContent::Description(text.to_string()),
                digest: None,
                registry: None,
            },
        )?;
    }
    Ok(bindings)
}

// The operator-owned file carrier: parsed fail-closed, never written
// by the engine, reached through `--config` or root discovery.
fn from_file(path: &Path) -> Result<Vec<SourceBinding>, Error> {
    let raw = std::fs::read_to_string(path)
        .map_err(|source| server_error!("{} ({source})", path.display()))?;
    let file: ConfigFile =
        toml::from_str(&raw).map_err(|err| bad_request!("{}: {err}", path.display()))?;
    if file.source.is_empty() {
        return Err(source_required());
    }
    let base = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut bindings = Vec::new();
    for entry in &file.source {
        push_unique(&mut bindings, binding(entry, base)?)?;
    }
    Ok(bindings)
}

fn binding(entry: &SourceEntry, base: &Path) -> Result<SourceBinding, Error> {
    let name = &entry.name;
    let selector = AdapterSelector::parse(&entry.adapter)?;
    let registry = registry(entry, &selector)?;
    let digest = digest(entry, &selector)?;
    let locations = [
        entry.path.is_some(),
        entry.git.is_some(),
        entry.url.is_some(),
        entry.description.is_some(),
    ];
    if locations.iter().filter(|present| **present).count() > 1 {
        return Err(bad_request!(
            "source `{name}` names more than one of `path`, `git`, `url`, `description`; exactly \
             one content key is allowed (omitted means the workspace lend at `.`)",
        ));
    }
    if let Some(remote) = entry.git.as_deref().or(entry.url.as_deref()) {
        if remote.starts_with("git+") {
            return Err(bad_request!(
                "source `{name}` uses Cargo's machine-written source-id form (`git+…`); write \
                 the plain URL with an optional `@ref` suffix",
            ));
        }
        return Err(bad_request!(
            "source `{name}` names a remote location (`git` / `url`); remote read views are \
             reserved and not yet supported — bind a local `path` or inline `description`",
        ));
    }
    let content = match (&entry.path, &entry.description) {
        (Some(relative), None) => {
            BindingContent::Workspace(resolved(base, Path::new(relative))?.display().to_string())
        }
        (None, Some(text)) => BindingContent::Description(text.clone()),
        (None, None) => BindingContent::Workspace(".".to_string()),
        (Some(_), Some(_)) => unreachable!("two content keys refused above"),
    };
    Ok(SourceBinding {
        key: name.clone(),
        adapter: adapter_value(&selector, &entry.adapter, base)?,
        content,
        digest,
        registry,
    })
}

// The endpoint override only steers registry acquisition, so it rides
// only a package-shaped selector.
fn registry(entry: &SourceEntry, selector: &AdapterSelector) -> Result<Option<String>, Error> {
    let name = &entry.name;
    let Some(endpoint) = entry.registry.as_deref() else {
        return Ok(None);
    };
    if !matches!(selector, AdapterSelector::Package { .. }) {
        return Err(bad_request!(
            "source `{name}` sets `registry` on an adapter the registry never serves; the \
             override only applies to registry package references (`<namespace>:<name>@<version>`)",
        ));
    }
    Ok(Some(endpoint.to_owned()))
}

// The pin binds exact component bytes, so it rides only a selector
// the loader acquires — a local component path or a registry package.
fn digest(entry: &SourceEntry, selector: &AdapterSelector) -> Result<Option<Digest>, Error> {
    let name = &entry.name;
    let Some(pin) = entry.digest.as_deref() else {
        return Ok(None);
    };
    if matches!(selector, AdapterSelector::Bare { .. }) {
        return Err(bad_request!(
            "source `{name}` sets `digest` on a bare adapter name the loader never acquires; \
             pin a local component path or an exact registry package reference instead",
        ));
    }
    pin.parse().map(Some).map_err(|err| bad_request!("source `{name}`: {err}",))
}

// A local component selector in the file resolves relative to the
// file, like Cargo `path` dependencies; other selector kinds pass
// through unchanged.
fn adapter_value(selector: &AdapterSelector, raw: &str, base: &Path) -> Result<String, Error> {
    match selector {
        AdapterSelector::Component { path } => Ok(resolved(base, path)?.display().to_string()),
        _ => Ok(raw.to_string()),
    }
}

// Anchor `relative` at the file's directory and normalise lexically,
// refusing any path outside the `.` project preopen.
fn resolved(base: &Path, relative: &Path) -> Result<PathBuf, Error> {
    preopen_path(&base.join(relative), "--config")
}

fn source_required() -> Error {
    Error::BadRequest {
        code: "specify-source-required".into(),
        description: "emery specify requires at least one source: pass `<adapter>` (package \
                      reference or local component path) and/or `--description \
                      <adapter>=<text>`, select an operator-owned config with `--config \
                      [<path>]`, or add an `emery.toml` at the project root"
            .into(),
    }
}

fn push_unique(bindings: &mut Vec<SourceBinding>, binding: SourceBinding) -> Result<(), Error> {
    if bindings.iter().any(|existing| existing.key == binding.key) {
        return Err(bad_request!(
            "each source binds once: source `{}` is bound twice",
            binding.key
        ));
    }
    bindings.push(binding);
    Ok(())
}

fn split_description(entry: &str) -> Result<(&str, &str), Error> {
    entry.split_once('=').filter(|(adapter, _)| !adapter.is_empty()).ok_or_else(|| {
        bad_request!("invalid argument --description: expected `<adapter>=<text>`, got `{entry}`",)
    })
}

// The operator-authored schema: ordered `[[source]]` entries whose
// `name` is the binding key, with exactly one optional content key.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    #[serde(default)]
    source: Vec<SourceEntry>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct SourceEntry {
    name: String,
    adapter: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    git: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    registry: Option<String>,
    #[serde(default)]
    digest: Option<String>,
}

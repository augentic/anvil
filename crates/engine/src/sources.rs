//! Per-run source bindings from argv or an operator-owned `sources.toml`.
//!
//! The binding list is an input, never engine state: nothing here is
//! persisted, and the engine never writes the sources file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::handler::{Error, bad_request, preopen_path, server_error};
use crate::resolve::AdapterSelector;

/// A source binding for one run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceBinding {
    /// Stable binding key.
    pub key: String,
    /// The adapter selector value.
    pub adapter: String,
    /// What the adapter extracts.
    pub content: BindingContent,
}

/// Workspace or inline source content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BindingContent {
    /// Project-relative read-only root; `.` binds the project.
    Workspace(String),
    /// Inline value; no filesystem view.
    Value(String),
}

/// Builds the run's binding list from `emery specify` inputs.
///
/// # Errors
///
/// Returns `argument` when `--sources` is mixed with positional
/// adapters or `--value` bindings, and propagates the argv and file
/// loader failures.
pub fn bindings(
    adapters: &[String], values: &[String], sources: Option<&str>,
) -> Result<Vec<SourceBinding>, Error> {
    match sources {
        Some(path) => {
            if !adapters.is_empty() || !values.is_empty() {
                return Err(bad_request(
                    "argument",
                    "invalid argument --sources: cannot be combined with positional `<adapter>` \
                     or `--value` bindings; the file carries the whole binding list",
                ));
            }
            from_file(&preopen_path(Path::new(path), "--sources")?)
        }
        None => from_argv(adapters, values),
    }
}

// Argv bindings: each positional adapter lends the workspace at `.`;
// each `--value` entry is inline. The key is the adapter name.
fn from_argv(adapters: &[String], values: &[String]) -> Result<Vec<SourceBinding>, Error> {
    if adapters.is_empty() && values.is_empty() {
        return Err(source_required());
    }
    let mut bindings = Vec::new();
    for value in adapters {
        let key = AdapterSelector::parse(value)?.name()?;
        push_unique(
            &mut bindings,
            SourceBinding {
                key,
                adapter: value.clone(),
                content: BindingContent::Workspace(".".to_string()),
            },
        )?;
    }
    for entry in values {
        let (adapter, text) = split_value(entry)?;
        let key = AdapterSelector::parse(adapter)?.name()?;
        push_unique(
            &mut bindings,
            SourceBinding {
                key,
                adapter: adapter.to_string(),
                content: BindingContent::Value(text.to_string()),
            },
        )?;
    }
    Ok(bindings)
}

// The operator-owned file carrier: parsed fail-closed, never written
// by the engine, and only ever reached through an explicit path.
fn from_file(path: &Path) -> Result<Vec<SourceBinding>, Error> {
    let raw = std::fs::read_to_string(path).map_err(|source| {
        server_error("filesystem-read", format!("filesystem-read: {} ({source})", path.display()))
    })?;
    let file: SourcesFile = toml::from_str(&raw).map_err(|err| {
        bad_request(
            "sources-toml-malformed",
            format!("sources-toml-malformed: {}: {err}", path.display()),
        )
    })?;
    if file.sources.is_empty() {
        return Err(source_required());
    }
    let base = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut bindings = Vec::new();
    for (key, entry) in &file.sources {
        bindings.push(binding(key, entry, base)?);
    }
    Ok(bindings)
}

fn binding(key: &str, entry: &SourceEntry, base: &Path) -> Result<SourceBinding, Error> {
    let locations =
        [entry.path.is_some(), entry.git.is_some(), entry.url.is_some(), entry.value.is_some()];
    if locations.iter().filter(|present| **present).count() > 1 {
        return Err(bad_request(
            "argument",
            format!(
                "invalid argument --sources: source `{key}` names more than one of `path`, \
                 `git`, `url`, `value`; exactly one location key is allowed (omitted means the \
                 workspace lend at `.`)"
            ),
        ));
    }
    if let Some(remote) = entry.git.as_deref().or(entry.url.as_deref()) {
        if remote.starts_with("git+") {
            return Err(bad_request(
                "argument",
                format!(
                    "invalid argument --sources: source `{key}` uses Cargo's machine-written \
                     source-id form (`git+…`); write the plain URL with an optional `@ref` suffix"
                ),
            ));
        }
        return Err(bad_request(
            "source-remote-unsupported",
            format!(
                "source-remote-unsupported: source `{key}` names a remote location (`git` / \
                 `url`); remote read views are reserved and not yet supported — bind a local \
                 `path` or inline `value`"
            ),
        ));
    }
    let content = match (&entry.path, &entry.value) {
        (Some(relative), None) => {
            BindingContent::Workspace(resolved(base, Path::new(relative))?.display().to_string())
        }
        (None, Some(text)) => BindingContent::Value(text.clone()),
        (None, None) => BindingContent::Workspace(".".to_string()),
        (Some(_), Some(_)) => unreachable!("two location keys refused above"),
    };
    Ok(SourceBinding {
        key: key.to_string(),
        adapter: adapter_value(&entry.adapter, base)?,
        content,
    })
}

// A local component selector in the file resolves relative to the
// file, like Cargo `path` dependencies; other selector kinds pass
// through unchanged.
fn adapter_value(raw: &str, base: &Path) -> Result<String, Error> {
    match AdapterSelector::parse(raw)? {
        AdapterSelector::Component { path } => Ok(resolved(base, &path)?.display().to_string()),
        _ => Ok(raw.to_string()),
    }
}

// Anchor `relative` at the file's directory and normalise lexically,
// refusing any path outside the `.` project preopen.
fn resolved(base: &Path, relative: &Path) -> Result<PathBuf, Error> {
    preopen_path(&base.join(relative), "--sources")
}

fn source_required() -> Error {
    bad_request(
        "specify-source-required",
        "specify-source-required: emery specify requires at least one source: pass `<adapter>` \
         (package reference or local component path) and/or `--value <adapter>=<text>`, or \
         select an operator-owned file with `--sources [<path>]`",
    )
}

fn push_unique(bindings: &mut Vec<SourceBinding>, binding: SourceBinding) -> Result<(), Error> {
    if bindings.iter().any(|existing| existing.key == binding.key) {
        return Err(bad_request(
            "specify-source-duplicate",
            format!(
                "specify-source-duplicate: each source binds once: source `{}` is bound twice",
                binding.key
            ),
        ));
    }
    bindings.push(binding);
    Ok(())
}

fn split_value(entry: &str) -> Result<(&str, &str), Error> {
    entry.split_once('=').filter(|(adapter, _)| !adapter.is_empty()).ok_or_else(|| {
        bad_request(
            "argument",
            format!("invalid argument --value: expected `<adapter>=<text>`, got `{entry}`"),
        )
    })
}

// The operator-authored schema: `[sources.<key>]` tables whose key is
// the binding key, with exactly one optional location key.
#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SourcesFile {
    #[serde(default)]
    sources: BTreeMap<String, SourceEntry>,
}

#[derive(Debug, serde::Deserialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
struct SourceEntry {
    adapter: String,
    #[serde(default)]
    path: Option<String>,
    #[serde(default)]
    git: Option<String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    value: Option<String>,
}

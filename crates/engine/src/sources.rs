//! Per-run source bindings from argv or an operator-owned `sources.toml`.
//!
//! The binding list is an input, never engine state: nothing here is
//! persisted, and the engine never writes the sources file.

use std::collections::BTreeMap;
use std::path::{Component, Path, PathBuf};

use emery_error::Error;

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
/// Returns [`Error::Argument`] when `--sources` is mixed with
/// positional adapters or `--value` bindings, and propagates the
/// argv and file loader failures.
pub fn bindings(
    adapters: &[String], values: &[String], sources: Option<&str>,
) -> Result<Vec<SourceBinding>, Error> {
    match sources {
        Some(path) => {
            if !adapters.is_empty() || !values.is_empty() {
                return Err(Error::Argument {
                    flag: "--sources",
                    detail: "cannot be combined with positional `<adapter>` or `--value` \
                             bindings; the file carries the whole binding list"
                        .to_string(),
                });
            }
            from_file(Path::new(path))
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
    let raw = std::fs::read_to_string(path).map_err(|source| Error::Filesystem {
        op: "read",
        path: path.to_path_buf(),
        source,
    })?;
    let file: SourcesFile = toml::from_str(&raw).map_err(|err| Error::Diag {
        code: "sources-toml-malformed",
        detail: format!("{}: {err}", path.display()),
    })?;
    if file.sources.is_empty() {
        return Err(source_required());
    }
    let base = path.parent().unwrap_or_else(|| Path::new("."));
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
        return Err(Error::Argument {
            flag: "--sources",
            detail: format!(
                "source `{key}` names more than one of `path`, `git`, `url`, `value`; exactly \
                 one location key is allowed (omitted means the workspace lend at `.`)"
            ),
        });
    }
    if let Some(remote) = entry.git.as_deref().or(entry.url.as_deref()) {
        if remote.starts_with("git+") {
            return Err(Error::Argument {
                flag: "--sources",
                detail: format!(
                    "source `{key}` uses Cargo's machine-written source-id form (`git+…`); \
                     write the plain URL with an optional `@ref` suffix"
                ),
            });
        }
        return Err(Error::Diag {
            code: "source-remote-unsupported",
            detail: format!(
                "source `{key}` names a remote location (`git` / `url`); remote read views are \
                 reserved and not yet supported — bind a local `path` or inline `value`"
            ),
        });
    }
    let content = match (&entry.path, &entry.value) {
        (Some(relative), None) => {
            BindingContent::Workspace(resolved(base, Path::new(relative)).display().to_string())
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
        AdapterSelector::Component { path } if !path.is_absolute() => {
            Ok(resolved(base, &path).display().to_string())
        }
        _ => Ok(raw.to_string()),
    }
}

// Anchor `relative` at the file's directory and normalise lexically —
// the file works from any invocation directory (§5.2); a root the
// deployment cannot read fails at read time, not here.
fn resolved(base: &Path, relative: &Path) -> PathBuf {
    let joined = base.join(relative);
    let mut parts: Vec<Component<'_>> = Vec::new();
    for component in joined.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => match parts.last() {
                Some(Component::Normal(_)) => {
                    let _ = parts.pop();
                }
                _ => parts.push(component),
            },
            other => parts.push(other),
        }
    }
    if parts.is_empty() { PathBuf::from(".") } else { parts.iter().collect() }
}

fn source_required() -> Error {
    Error::validation_failed(
        "specify-source-required",
        "emery specify requires at least one source",
        "pass `<adapter>` (package reference or local component path) and/or `--value \
         <adapter>=<text>`, or point at an operator-owned file with `--sources <path>`",
    )
}

fn push_unique(bindings: &mut Vec<SourceBinding>, binding: SourceBinding) -> Result<(), Error> {
    if bindings.iter().any(|existing| existing.key == binding.key) {
        return Err(Error::validation_failed(
            "specify-source-duplicate",
            "each source binds once",
            format!("source `{}` is bound twice", binding.key),
        ));
    }
    bindings.push(binding);
    Ok(())
}

fn split_value(entry: &str) -> Result<(&str, &str), Error> {
    entry.split_once('=').filter(|(adapter, _)| !adapter.is_empty()).ok_or_else(|| {
        Error::Argument {
            flag: "--value",
            detail: format!("expected `<adapter>=<text>`, got `{entry}`"),
        }
    })
}

// The operator-authored schema: `[sources.<key>]` tables whose key is
// the seam binding key, with exactly one optional location key.
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

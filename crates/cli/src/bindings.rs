//! The binding carriers: argv positionals plus `--description`, the
//! operator-owned `emery.toml` named by `--config`, and project-root
//! discovery for a run naming no bindings at all.

use std::path::{Path, PathBuf};

use emery_engine::sources::{BindingContent, SourceBinding};
use emery_engine::{AdapterRef, preopen_path};
use omnia_guest::{Error, bad_request, server_error};

/// The project-root config discovered by a bindingless run.
pub const CONFIG_FILE: &str = "emery.toml";

/// Decodes the run's binding list from the `specify` arguments.
///
/// # Errors
///
/// Returns a `BadRequest` when `--config` is mixed with positional
/// adapters or `--description` bindings, and propagates the argv,
/// file, and discovery decoder failures.
pub fn decode(
    adapters: &[String], descriptions: &[String], config: Option<&str>,
) -> Result<Vec<SourceBinding>, Error> {
    match config {
        Some(path) => {
            if !adapters.is_empty() || !descriptions.is_empty() {
                return Err(bad_request!(
                    "--config cannot be combined with `<adapter>` or `--description`"
                ));
            }
            let path = preopen_path(Path::new(path)).map_err(|err| {
                let description = err.description();
                bad_request!("invalid argument --config: {description}")
            })?;
            from_file(&path)
        }
        None if adapters.is_empty() && descriptions.is_empty() => discover(),
        None => from_argv(adapters, descriptions),
    }
}

// A missing project-root file yields the empty list the engine refuses
// typed; a parse failure still refuses typed.
fn discover() -> Result<Vec<SourceBinding>, Error> {
    let path = Path::new(CONFIG_FILE);
    let present =
        path.try_exists().map_err(|source| server_error!("reading {CONFIG_FILE}: {source}"))?;
    if present { from_file(path) } else { Ok(Vec::new()) }
}

// Each positional adapter lends the workspace at `.`; each
// `--description` entry is inline. The key is the adapter name.
fn from_argv(adapters: &[String], descriptions: &[String]) -> Result<Vec<SourceBinding>, Error> {
    let mut bindings = Vec::new();
    for value in adapters {
        bindings.push(SourceBinding {
            key: value.parse::<AdapterRef>()?.name()?,
            adapter: value.clone(),
            content: BindingContent::Workspace(".".to_string()),
            digest: None,
            registry: None,
        });
    }

    for entry in descriptions {
        let (adapter, text) =
            entry.split_once('=').filter(|(adapter, _)| !adapter.is_empty()).ok_or_else(|| {
                bad_request!(
                    "invalid argument --description: expected `<adapter>=<text>`, got `{entry}`"
                )
            })?;
        bindings.push(SourceBinding {
            key: adapter.parse::<AdapterRef>()?.name()?,
            adapter: adapter.to_string(),
            content: BindingContent::Description(text.to_string()),
            digest: None,
            registry: None,
        });
    }

    Ok(bindings)
}

// The operator-owned file: parsed fail-closed, never written by the engine.
fn from_file(path: &Path) -> Result<Vec<SourceBinding>, Error> {
    let raw = std::fs::read_to_string(path).map_err(|source| {
        let path = path.display();
        server_error!("reading {path}: {source}")
    })?;
    let file: ConfigFile = toml::from_str(&raw).map_err(|err| {
        let path = path.display();
        bad_request!("{path}: {err}")
    })?;

    let base = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    file.source.iter().map(|entry| binding(entry, base)).collect()
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

fn binding(entry: &SourceEntry, base: &Path) -> Result<SourceBinding, Error> {
    let name = &entry.name;
    let selector: AdapterRef = entry.adapter.parse()?;
    let digest = entry
        .digest
        .as_deref()
        .map(|pin| pin.parse().map_err(|err| bad_request!("source `{name}`: {err}")))
        .transpose()?;

    let locations = [
        entry.path.is_some(),
        entry.git.is_some(),
        entry.url.is_some(),
        entry.description.is_some(),
    ];
    if locations.iter().filter(|present| **present).count() > 1 {
        return Err(bad_request!(
            "source `{name}` sets more than one of `path`, `git`, `url`, `description`"
        ));
    }
    if let Some(remote) = entry.git.as_deref().or(entry.url.as_deref()) {
        if remote.starts_with("git+") {
            return Err(bad_request!(
                "source `{name}`: drop the `git+` prefix and write the plain URL"
            ));
        }
        return Err(bad_request!(
            "source `{name}`: `git` and `url` are not supported; use `path` or `description`"
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
    // A local component path resolves relative to the file, like Cargo
    // `path` dependencies; other selector kinds pass through unchanged.
    let adapter = match &selector {
        AdapterRef::Component(path) => resolved(base, path)?.display().to_string(),
        _ => entry.adapter.clone(),
    };

    Ok(SourceBinding {
        key: name.clone(),
        adapter,
        content,
        digest,
        registry: entry.registry.clone(),
    })
}

// Anchors `relative` at the file's directory, refusing any path outside
// the `.` project preopen.
fn resolved(base: &Path, relative: &Path) -> Result<PathBuf, Error> {
    preopen_path(&base.join(relative))
}

//! Ergonomic load → mutate → atomic-write loop for `.specify/` YAML
//! state. [`AtomicYaml`] pairs state with its [`Layout`] location;
//! [`with_state`] wraps the load and atomic-write halves.

use std::path::PathBuf;

use serde::Serialize;
use serde::de::DeserializeOwned;
use specify_error::Error;
use specify_model::atomic::yaml_write;

use crate::config::{Layout, ProjectConfig};
use crate::registry::Registry;

/// A piece of `.specify/`-anchored YAML state.
///
/// Implementors pair a canonical on-disk location with atomic-write
/// semantics by exposing a `load` / `path` pair so [`with_state`] can
/// wrap the load → mutate → write loop.
pub trait AtomicYaml: Sized + Serialize + DeserializeOwned {
    /// Path under [`Layout`] where this state lives.
    ///
    /// Named distinctly from any inherent `path` so trait and inherent
    /// methods never collide on the implementing type.
    fn layout_path(layout: Layout<'_>) -> PathBuf;

    /// Load from disk. Default implementation reads [`Self::layout_path`],
    /// deserialises, and returns `Ok(None)` when the file is absent.
    /// Override when the state needs validation at load time
    /// (the existing `Registry::load` runs `validate_shape` before
    /// returning).
    ///
    /// # Errors
    ///
    /// Propagates I/O failures and YAML parse errors.
    fn load_state(layout: Layout<'_>) -> Result<Option<Self>, Error> {
        let path = Self::layout_path(layout);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&path)?;
        let value: Self = serde_saphyr::from_str(&content)?;
        Ok(Some(value))
    }
}

/// Load → mutate → atomic-write loop.
///
/// Loads `S` from disk, returning [`Error::ArtifactNotFound`] with
/// `missing_kind` when the file is absent. Runs `f` against the
/// in-memory state, atomically writes the mutated value back, then
/// returns the body the closure produced.
///
/// `with_state` does **not** itself emit; the caller writes
/// `ctx.write(&body, write_text)?;`. This keeps response shaping local
/// to each handler and the helper focused on the IO loop.
///
/// Handlers whose contract is "create or update" (e.g. `registry add`)
/// inline their own load-or-default + [`yaml_write`] instead of
/// reaching for this helper.
///
/// # Errors
///
/// - Returns [`Error::ArtifactNotFound`] when the file is absent.
/// - Otherwise propagates errors from `load`, the closure, and the
///   atomic write.
pub fn with_state<S, B, F>(layout: Layout<'_>, missing_kind: &'static str, f: F) -> Result<B, Error>
where
    S: AtomicYaml,
    F: FnOnce(&mut S) -> Result<B, Error>,
{
    let path = S::layout_path(layout);
    let mut state = S::load_state(layout)?.ok_or_else(|| Error::ArtifactNotFound {
        kind: missing_kind,
        path: path.clone(),
    })?;
    let body = f(&mut state)?;
    yaml_write(&path, &state)?;
    Ok(body)
}

impl AtomicYaml for Registry {
    fn layout_path(layout: Layout<'_>) -> PathBuf {
        layout.registry_path()
    }

    /// Delegate to the inherent loader so `validate_shape` runs at
    /// load time — the trait's default impl would skip that.
    fn load_state(layout: Layout<'_>) -> Result<Option<Self>, Error> {
        Self::load(layout.project_dir())
    }
}

impl AtomicYaml for ProjectConfig {
    fn layout_path(layout: Layout<'_>) -> PathBuf {
        layout.config_path()
    }

    /// Delegate to the inherent loader so the `specify_version` floor
    /// check runs at load time. Map the canonical "absent" error
    /// ([`Error::NotInitialized`]) to `Ok(None)` so the trait's
    /// "absent → None" contract holds; callers that need the typed
    /// error should call [`ProjectConfig::load`] directly.
    fn load_state(layout: Layout<'_>) -> Result<Option<Self>, Error> {
        match Self::load(layout.project_dir()) {
            Ok(cfg) => Ok(Some(cfg)),
            Err(Error::NotInitialized) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
mod tests;

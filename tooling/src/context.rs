use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use jsonschema::Validator;
use serde_json::Value as JsonValue;

use crate::error::ToolingError;

/// Shared scan context: framework root, schema cache, and optional CLI checkout hook.
pub struct Context {
    framework_root: PathBuf,
    schema_cache: Mutex<HashMap<PathBuf, Arc<Validator>>>,
}

impl Context {
    /// Resolve framework root from the process working directory.
    pub fn discover() -> Result<Self, ToolingError> {
        let cwd = env::current_dir()?;
        Self::from_start_dir(&cwd)
    }

    /// Resolve framework root relative to the tooling crate manifest directory.
    pub fn from_manifest_dir(manifest_dir: impl AsRef<Path>) -> Result<Self, ToolingError> {
        Self::from_start_dir(manifest_dir.as_ref())
    }

    /// Framework repo root — parent of `tooling/`, never `tooling/` itself.
    pub fn framework_root(&self) -> &Path {
        &self.framework_root
    }

    /// `plugins/` under the framework root.
    pub fn plugins_dir(&self) -> PathBuf {
        self.framework_root.join("plugins")
    }

    /// `adapters/sources/` under the framework root.
    pub fn sources_dir(&self) -> PathBuf {
        self.framework_root.join("adapters").join("sources")
    }

    /// `adapters/targets/` under the framework root.
    pub fn targets_dir(&self) -> PathBuf {
        self.framework_root.join("adapters").join("targets")
    }

    /// `adapters/shared/` under the framework root.
    pub fn adapters_shared_dir(&self) -> PathBuf {
        self.framework_root.join("adapters").join("shared")
    }

    /// `.cursor/schemas/` under the framework root.
    pub fn cursor_schema_dir(&self) -> PathBuf {
        self.framework_root.join(".cursor").join("schemas")
    }

    /// Runtime JSON Schemas from a sibling or overridden `specify-cli` checkout.
    ///
    /// Mirrors `scripts/checks/_shared.ts` `resolveSpecifyCliSchemasDir()`:
    /// `join(framework_root, SPECIFY_CLI_DIR ?? "../specify-cli", "schemas")`.
    pub fn specify_cli_schemas_dir(&self) -> PathBuf {
        let checkout = env::var("SPECIFY_CLI_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("../specify-cli"));
        let checkout = if checkout.is_absolute() {
            checkout
        } else {
            self.framework_root.join(checkout)
        };
        checkout.join("schemas")
    }

    /// Lazily compile and cache a JSON Schema loaded from `path`.
    pub fn schema(&self, path: impl AsRef<Path>) -> Result<Arc<Validator>, ToolingError> {
        let path = path.as_ref().to_path_buf();
        let mut cache = self
            .schema_cache
            .lock()
            .map_err(|_| ToolingError::Infrastructure("schema cache poisoned".into()))?;

        if let Some(schema) = cache.get(&path) {
            return Ok(Arc::clone(schema));
        }

        let contents = std::fs::read_to_string(&path).map_err(|source| {
            ToolingError::Infrastructure(format!("read schema {}: {source}", path.display()))
        })?;
        let value: JsonValue = serde_json::from_str(&contents).map_err(|source| {
            ToolingError::Infrastructure(format!("parse schema {}: {source}", path.display()))
        })?;
        let compiled = jsonschema::validator_for(&value).map_err(|error| {
            ToolingError::Infrastructure(format!("compile schema {}: {error}", path.display()))
        })?;
        let compiled = Arc::new(compiled);
        cache.insert(path, Arc::clone(&compiled));
        Ok(compiled)
    }

    fn from_start_dir(start: &Path) -> Result<Self, ToolingError> {
        let framework_root = resolve_framework_root(start)?;
        Ok(Self {
            framework_root,
            schema_cache: Mutex::new(HashMap::new()),
        })
    }
}

fn resolve_framework_root(start: &Path) -> Result<PathBuf, ToolingError> {
    let start = start
        .canonicalize()
        .map_err(|source| ToolingError::Infrastructure(format!("canonicalize path: {source}")))?;

    let mut dir = start.clone();
    loop {
        if is_framework_root(&dir) {
            return Ok(dir);
        }

        if dir.file_name().is_some_and(|name| name == "tooling") {
            if let Some(parent) = dir.parent() {
                if is_framework_root(parent) {
                    return Ok(parent.to_path_buf());
                }
            }
        }

        dir = dir
            .parent()
            .ok_or_else(|| {
                ToolingError::Infrastructure(format!(
                    "framework root not found from {}",
                    start.display()
                ))
            })?
            .to_path_buf();
    }
}

fn is_framework_root(path: &Path) -> bool {
    path.join("tooling").join("Cargo.toml").is_file()
        && path.join("plugins").is_dir()
        && path.join("adapters").is_dir()
}

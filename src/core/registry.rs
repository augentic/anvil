//! Schema registry -- resolves schemas from embedded, local store, or GitHub.

use std::path::Path;

use anyhow::{Context, Result, bail};
use include_dir::Dir;

use super::embedded::EMBEDDED_SCHEMAS;
use super::paths::DataDir;
use super::schema::Schema;

/// Metadata about a discovered schema (without loading full content).
#[derive(Debug, Clone)]
pub struct SchemaEntry {
    /// Schema name (directory name).
    pub name: String,
    /// Where this schema was found.
    pub source: SchemaSource,
    /// Parsed schema metadata.
    pub schema: Schema,
}

/// Where a schema was resolved from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchemaSource {
    /// Compiled into the binary.
    Embedded,
    /// Found in `~/.local/share/openspec/schemas/`.
    LocalStore,
    /// Found in the current project's `openspec/schemas/`.
    Project,
}

impl std::fmt::Display for SchemaSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Embedded => f.write_str("embedded"),
            Self::LocalStore => f.write_str("local store"),
            Self::Project => f.write_str("project"),
        }
    }
}

/// List all schemas available from embedded data.
#[must_use]
pub fn list_embedded() -> Vec<SchemaEntry> {
    embedded_entries(&EMBEDDED_SCHEMAS, SchemaSource::Embedded)
}

/// List all schemas available from the local data store.
///
/// # Errors
///
/// Returns an error if the data directory cannot be resolved or read.
pub fn list_local_store() -> Result<Vec<SchemaEntry>> {
    let data_dir = DataDir::resolve()?;
    let schemas_dir = data_dir.schemas_dir();
    if !schemas_dir.is_dir() {
        return Ok(Vec::new());
    }
    entries_from_dir(&schemas_dir, SchemaSource::LocalStore)
}

/// List all schemas available from the current project.
///
/// # Errors
///
/// Returns an error if the directory cannot be read.
pub fn list_project(project_schemas_dir: &Path) -> Result<Vec<SchemaEntry>> {
    if !project_schemas_dir.is_dir() {
        return Ok(Vec::new());
    }
    entries_from_dir(project_schemas_dir, SchemaSource::Project)
}

/// Resolve a schema by name using the priority order: local store > embedded.
///
/// Returns the schema directory contents and metadata. The caller decides
/// where to copy files.
///
/// # Errors
///
/// Returns an error if the schema cannot be found in any source.
pub fn resolve(name: &str) -> Result<ResolvedSchema> {
    let data_dir = DataDir::resolve()?;
    let local_path = data_dir.schema_dir(name);
    if local_path.is_dir() {
        let schema_yaml = std::fs::read(local_path.join("schema.yaml"))
            .with_context(|| format!("reading schema.yaml from {}", local_path.display()))?;
        let schema = Schema::from_yaml(&schema_yaml)?;
        return Ok(ResolvedSchema {
            source: SchemaSource::LocalStore,
            schema,
            location: SchemaLocation::Filesystem(local_path),
        });
    }

    if let Some(dir) = EMBEDDED_SCHEMAS.get_dir(name) {
        let schema_file = dir
            .get_file(format!("{name}/schema.yaml"))
            .or_else(|| dir.get_file("schema.yaml"))
            .context("embedded schema missing schema.yaml")?;
        let schema = Schema::from_yaml(schema_file.contents())?;
        return Ok(ResolvedSchema {
            source: SchemaSource::Embedded,
            schema,
            location: SchemaLocation::Embedded(name.to_string()),
        });
    }

    bail!(
        "schema '{name}' not found in local store or embedded schemas; run `specify update` to fetch it"
    );
}

/// A schema that has been located but not yet copied.
#[derive(Debug)]
pub struct ResolvedSchema {
    /// Where it was found.
    pub source: SchemaSource,
    /// Parsed schema metadata.
    pub schema: Schema,
    /// How to access the schema files.
    pub location: SchemaLocation,
}

/// How to access a resolved schema's files.
#[derive(Debug)]
pub enum SchemaLocation {
    /// On disk at this path.
    Filesystem(std::path::PathBuf),
    /// Embedded in the binary under this schema name.
    Embedded(String),
}

impl ResolvedSchema {
    /// Copy the schema's directory tree into `dest`.
    ///
    /// `dest` should be the target schema directory (e.g. `openspec/schemas/omnia/`).
    ///
    /// # Errors
    ///
    /// Returns an error if files cannot be created or written.
    pub fn copy_to(&self, dest: &Path) -> Result<()> {
        std::fs::create_dir_all(dest).with_context(|| format!("creating {}", dest.display()))?;

        match &self.location {
            SchemaLocation::Filesystem(src) => copy_dir_recursive(src, dest),
            SchemaLocation::Embedded(name) => {
                let dir = EMBEDDED_SCHEMAS
                    .get_dir(name)
                    .with_context(|| format!("embedded schema '{name}' not found"))?;
                write_embedded_dir(dir, dest, name)
            }
        }
    }
}

/// Recursively copy a directory.
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    for entry in
        std::fs::read_dir(src).with_context(|| format!("reading directory {}", src.display()))?
    {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let target = dest.join(entry.file_name());

        if file_type.is_dir() {
            std::fs::create_dir_all(&target)?;
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Write an embedded `Dir` to the filesystem, stripping the schema-name prefix
/// from paths.
fn write_embedded_dir(dir: &Dir<'_>, dest: &Path, schema_name: &str) -> Result<()> {
    let prefix = format!("{schema_name}/");
    for file in dir.files() {
        let rel = file
            .path()
            .to_str()
            .unwrap_or_default()
            .strip_prefix(&prefix)
            .unwrap_or_else(|| file.path().to_str().unwrap_or_default());
        let target = dest.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&target, file.contents())
            .with_context(|| format!("writing {}", target.display()))?;
    }

    for subdir in dir.dirs() {
        write_embedded_dir(subdir, dest, schema_name)?;
    }
    Ok(())
}

/// Build `SchemaEntry` list from embedded Dir children.
fn embedded_entries(root: &Dir<'_>, source: SchemaSource) -> Vec<SchemaEntry> {
    root.dirs()
        .filter_map(|dir| {
            let name = dir.path().file_name()?.to_str()?.to_string();
            let schema_file = dir
                .get_file(format!("{name}/schema.yaml"))
                .or_else(|| dir.get_file("schema.yaml"))?;
            let schema = Schema::from_yaml(schema_file.contents()).ok()?;
            Some(SchemaEntry { name, source, schema })
        })
        .collect()
}

/// Build `SchemaEntry` list from a filesystem directory.
fn entries_from_dir(dir: &Path, source: SchemaSource) -> Result<Vec<SchemaEntry>> {
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let schema_path = entry.path().join("schema.yaml");
        if !schema_path.is_file() {
            continue;
        }
        let bytes = std::fs::read(&schema_path)?;
        if let Ok(schema) = Schema::from_yaml(&bytes) {
            let name = entry.file_name().to_str().unwrap_or_default().to_string();
            entries.push(SchemaEntry { name, source, schema });
        }
    }
    Ok(entries)
}

/// Fetch schemas from a GitHub repository and write them to the local data store.
///
/// Uses the GitHub Contents API to recursively download the `openspec/schemas/`
/// directory from the given repository.
///
/// # Errors
///
/// Returns an error if the GitHub API request fails or files cannot be written.
pub fn fetch_from_github(repo: &str, git_ref: &str) -> Result<Vec<String>> {
    let data_dir = DataDir::resolve()?;
    data_dir.ensure()?;

    let schemas = discover_github_schemas(repo, git_ref)?;
    let mut updated = Vec::new();

    for schema_name in &schemas {
        let dest = data_dir.schema_dir(schema_name);
        std::fs::create_dir_all(&dest)?;
        download_github_dir(repo, git_ref, &format!("openspec/schemas/{schema_name}"), &dest)?;
        updated.push(schema_name.clone());
    }

    Ok(updated)
}

/// Discover which schema directories exist in the remote repo.
fn discover_github_schemas(repo: &str, git_ref: &str) -> Result<Vec<String>> {
    let url =
        format!("https://api.github.com/repos/{repo}/contents/openspec/schemas?ref={git_ref}");
    let response: Vec<GitHubContent> = github_get_json(&url)?;
    Ok(response.into_iter().filter(|c| c.content_type == "dir").map(|c| c.name).collect())
}

/// Recursively download a directory from GitHub's Contents API.
fn download_github_dir(repo: &str, git_ref: &str, path: &str, dest: &Path) -> Result<()> {
    let url = format!("https://api.github.com/repos/{repo}/contents/{path}?ref={git_ref}");
    let entries: Vec<GitHubContent> = github_get_json(&url)?;

    for entry in entries {
        let target = dest.join(&entry.name);
        match entry.content_type.as_str() {
            "file" => {
                let download_url = entry
                    .download_url
                    .as_deref()
                    .with_context(|| format!("no download URL for {}", entry.path))?;
                let body = ureq::get(download_url)
                    .call()
                    .with_context(|| format!("downloading {}", entry.path))?
                    .into_body()
                    .read_to_string()
                    .with_context(|| format!("reading response for {}", entry.path))?;
                std::fs::write(&target, body)
                    .with_context(|| format!("writing {}", target.display()))?;
            }
            "dir" => {
                std::fs::create_dir_all(&target)?;
                download_github_dir(repo, git_ref, &entry.path, &target)?;
            }
            _ => {}
        }
    }
    Ok(())
}

/// Minimal GitHub Contents API response.
#[derive(Debug, serde::Deserialize)]
struct GitHubContent {
    name: String,
    path: String,
    #[serde(rename = "type")]
    content_type: String,
    download_url: Option<String>,
}

/// GET JSON from the GitHub API with a User-Agent header.
fn github_get_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T> {
    let body = ureq::get(url)
        .header("User-Agent", concat!("specify/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github.v3+json")
        .call()
        .with_context(|| format!("GitHub API request to {url}"))?
        .into_body()
        .read_to_string()
        .with_context(|| format!("reading GitHub response from {url}"))?;
    serde_json::from_str(&body).with_context(|| format!("parsing GitHub response from {url}"))
}

//! Change lifecycle -- creation, discovery, metadata, and archival.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::paths::ProjectDir;

const METADATA_FILE: &str = ".metadata.yaml";

/// An active or archived change directory with its parsed metadata.
#[derive(Debug, Clone)]
pub struct Change {
    /// Kebab-case name (directory name).
    pub name: String,
    /// Absolute path to the change directory.
    pub path: PathBuf,
    /// Parsed contents of `.metadata.yaml`.
    pub metadata: ChangeMetadata,
}

/// Persistent metadata stored alongside each change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeMetadata {
    /// Schema name that governs this change's artifact set.
    pub schema: String,
    /// ISO-8601 timestamp when the change was created.
    pub created_at: String,
}

impl Change {
    /// Create a new change directory with metadata.
    ///
    /// # Errors
    ///
    /// Returns an error if the name is invalid, a change with the same name
    /// already exists, or filesystem operations fail.
    pub fn create(project: &ProjectDir, name: &str, schema: &str) -> Result<Self> {
        validate_name(name)?;

        let change_dir = project.change_dir(name);
        if change_dir.exists() {
            bail!("change '{name}' already exists");
        }

        let specs_dir = change_dir.join("specs");
        std::fs::create_dir_all(&specs_dir)
            .with_context(|| format!("creating {}", specs_dir.display()))?;

        let metadata = ChangeMetadata {
            schema: schema.to_string(),
            created_at: Utc::now().to_rfc3339(),
        };

        let metadata_path = change_dir.join(METADATA_FILE);
        let yaml = serde_yaml::to_string(&metadata).context("serializing change metadata")?;
        std::fs::write(&metadata_path, yaml)
            .with_context(|| format!("writing {}", metadata_path.display()))?;

        Ok(Self {
            name: name.to_string(),
            path: change_dir,
            metadata,
        })
    }

    /// Discover all active (non-archived) changes under `specify/changes/`.
    ///
    /// # Errors
    ///
    /// Returns an error if the changes directory cannot be read.
    pub fn discover_active(project: &ProjectDir) -> Result<Vec<Self>> {
        let changes_dir = project.changes_dir();
        if !changes_dir.is_dir() {
            return Ok(Vec::new());
        }
        discover_in(&changes_dir, true)
    }

    /// Discover all archived changes under `specify/changes/archive/`.
    ///
    /// # Errors
    ///
    /// Returns an error if the archive directory cannot be read.
    pub fn discover_archived(project: &ProjectDir) -> Result<Vec<Self>> {
        let archive_dir = project.archive_dir();
        if !archive_dir.is_dir() {
            return Ok(Vec::new());
        }
        discover_in(&archive_dir, false)
    }

    /// Move this change into the dated archive directory.
    ///
    /// Returns the new path inside `specify/changes/archive/<date>-<name>/`.
    ///
    /// # Errors
    ///
    /// Returns an error if the archive directory cannot be created or the
    /// rename fails (e.g. cross-device move).
    pub fn archive(self, project: &ProjectDir) -> Result<PathBuf> {
        let date = Utc::now().format("%Y-%m-%d");
        let archive_name = format!("{date}-{}", self.name);
        let dest = project.archive_dir().join(&archive_name);

        std::fs::create_dir_all(project.archive_dir())
            .with_context(|| format!("creating {}", project.archive_dir().display()))?;

        if dest.exists() {
            bail!("archive destination already exists: {}", dest.display());
        }

        rename_dir(&self.path, &dest)?;
        Ok(dest)
    }

    /// Resolve a single active change by name, or auto-select if there is exactly one.
    ///
    /// When `name` is `None` and exactly one active change exists it is returned
    /// automatically. This cannot panic because the `len == 1` invariant is
    /// checked before calling [`Iterator::next`].
    ///
    /// # Errors
    ///
    /// Returns an error if the name doesn't match any active change, or if no
    /// name is provided and there isn't exactly one active change.
    pub fn resolve(project: &ProjectDir, name: Option<&str>) -> Result<Self> {
        let active = Self::discover_active(project)?;

        if let Some(name) = name {
            return active
                .into_iter()
                .find(|c| c.name == name)
                .with_context(|| format!("no active change named '{name}'"));
        }

        if active.is_empty() {
            bail!("no active changes found");
        }
        if active.len() > 1 {
            bail!(
                "{} active changes found; specify which one: {}",
                active.len(),
                active.iter().map(|c| c.name.as_str()).collect::<Vec<_>>().join(", ")
            );
        }
        // Exactly one element after the guards above.
        active.into_iter().next().context("no active changes")
    }
}

/// Discover changes in a directory, skipping the `archive/` subdirectory when
/// `skip_archive` is true.
fn discover_in(dir: &Path, skip_archive: bool) -> Result<Vec<Change>> {
    let mut changes = Vec::new();

    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if skip_archive && name == "archive" {
            continue;
        }

        let metadata_path = entry.path().join(METADATA_FILE);
        let Some(metadata) = load_metadata(&metadata_path) else {
            continue;
        };

        changes.push(Change {
            name,
            path: entry.path(),
            metadata,
        });
    }

    changes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(changes)
}

fn load_metadata(path: &Path) -> Option<ChangeMetadata> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_yaml::from_str(&content).ok()
}

/// Validate that a change name is kebab-case (lowercase alphanumeric + hyphens).
fn validate_name(name: &str) -> Result<()> {
    if name.is_empty() {
        bail!("change name cannot be empty");
    }
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        bail!("change name must be kebab-case (lowercase letters, digits, hyphens): '{name}'");
    }
    if name.starts_with('-') || name.ends_with('-') {
        bail!("change name must not start or end with a hyphen: '{name}'");
    }
    Ok(())
}

/// Rename a directory, falling back to recursive copy + delete for cross-device moves.
fn rename_dir(src: &Path, dest: &Path) -> Result<()> {
    if std::fs::rename(src, dest).is_ok() {
        return Ok(());
    }

    copy_dir_recursive(src, dest)?;
    std::fs::remove_dir_all(src).with_context(|| format!("removing {}", src.display()))
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    std::fs::create_dir_all(dest).with_context(|| format!("creating {}", dest.display()))?;

    for entry in std::fs::read_dir(src).with_context(|| format!("reading {}", src.display()))? {
        let entry = entry?;
        let target = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), &target)
                .with_context(|| format!("copying {}", entry.path().display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_project(tmp: &Path) -> ProjectDir {
        let project = ProjectDir::from_root(tmp);
        project.ensure().unwrap();
        project
    }

    #[test]
    fn create_change_writes_metadata() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project = setup_project(tmp.path());

        let change = Change::create(&project, "add-caching", "omnia").unwrap();

        assert_eq!(change.name, "add-caching");
        assert_eq!(change.metadata.schema, "omnia");
        assert!(!change.metadata.created_at.is_empty());
        assert!(change.path.join(".metadata.yaml").is_file());
        assert!(change.path.join("specs").is_dir());
    }

    #[test]
    fn duplicate_name_rejected() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project = setup_project(tmp.path());

        Change::create(&project, "my-change", "omnia").unwrap();
        let result = Change::create(&project, "my-change", "omnia");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn invalid_names_rejected() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project = setup_project(tmp.path());

        Change::create(&project, "", "s").unwrap_err();
        Change::create(&project, "Has Spaces", "s").unwrap_err();
        Change::create(&project, "UPPERCASE", "s").unwrap_err();
        Change::create(&project, "-leading", "s").unwrap_err();
        Change::create(&project, "trailing-", "s").unwrap_err();

        Change::create(&project, "valid-name-123", "s").unwrap();
    }

    #[test]
    fn discover_active_skips_archive() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project = setup_project(tmp.path());

        Change::create(&project, "active-one", "omnia").unwrap();
        Change::create(&project, "active-two", "omnia").unwrap();

        // Create a fake archive directory
        let archive = project.archive_dir().join("2026-01-01-old-change");
        std::fs::create_dir_all(&archive).unwrap();
        let meta = ChangeMetadata {
            schema: "omnia".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        std::fs::write(archive.join(".metadata.yaml"), serde_yaml::to_string(&meta).unwrap())
            .unwrap();

        let active = Change::discover_active(&project).unwrap();
        assert_eq!(active.len(), 2);
        assert!(active.iter().all(|c| c.name != "archive"));

        let archived = Change::discover_archived(&project).unwrap();
        assert_eq!(archived.len(), 1);
        assert_eq!(archived[0].name, "2026-01-01-old-change");
    }

    #[test]
    fn resolve_single_change() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project = setup_project(tmp.path());

        Change::create(&project, "only-one", "omnia").unwrap();
        let resolved = Change::resolve(&project, None).unwrap();
        assert_eq!(resolved.name, "only-one");
    }

    #[test]
    fn resolve_multiple_without_name_is_error() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project = setup_project(tmp.path());

        Change::create(&project, "first", "omnia").unwrap();
        Change::create(&project, "second", "omnia").unwrap();

        let result = Change::resolve(&project, None);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("specify which one"));
    }

    #[test]
    fn archive_moves_to_dated_dir() {
        let tmp = tempfile::TempDir::new().unwrap();
        let project = setup_project(tmp.path());

        let change = Change::create(&project, "to-archive", "omnia").unwrap();
        std::fs::write(change.path.join("proposal.md"), "done").unwrap();

        let archived_path = change.archive(&project).unwrap();
        assert!(archived_path.exists());
        assert!(!project.change_dir("to-archive").exists());
        assert!(archived_path.join("proposal.md").is_file());
        assert!(archived_path.join(".metadata.yaml").is_file());
    }
}

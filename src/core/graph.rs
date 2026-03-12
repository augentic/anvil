//! Artifact dependency graph -- topological ordering and filesystem-based completion tracking.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use anyhow::{Result, bail};
use serde::Serialize;

use super::schema::Schema;

/// Directed acyclic graph of artifact dependencies derived from a [`Schema`].
#[derive(Debug)]
pub struct ArtifactGraph {
    /// Artifact IDs in schema-declared order.
    ids: Vec<String>,
    /// `generates` pattern for each artifact, keyed by ID.
    generates: HashMap<String, String>,
    /// Forward edges: artifact -> artifacts that depend on it.
    dependents: HashMap<String, Vec<String>>,
    /// Reverse edges: artifact -> artifacts it depends on.
    dependencies: HashMap<String, Vec<String>>,
    /// Apply-phase metadata (if the schema defines one).
    apply_requires: Vec<String>,
}

impl ArtifactGraph {
    /// Build a graph from a schema definition.
    ///
    /// # Errors
    ///
    /// Returns an error if a `requires` reference points to an unknown artifact
    /// or the graph contains a cycle.
    pub fn from_schema(schema: &Schema) -> Result<Self> {
        let known: HashSet<&str> = schema.artifacts.iter().map(|a| a.id.as_str()).collect();

        let mut dependents: HashMap<String, Vec<String>> = HashMap::new();
        let mut dependencies: HashMap<String, Vec<String>> = HashMap::new();
        let mut generates = HashMap::new();

        for artifact in &schema.artifacts {
            generates.insert(artifact.id.clone(), artifact.generates.clone());
            dependencies.insert(artifact.id.clone(), artifact.requires.clone());

            for req in &artifact.requires {
                if !known.contains(req.as_str()) {
                    bail!("artifact '{}' requires unknown artifact '{req}'", artifact.id);
                }
                dependents.entry(req.clone()).or_default().push(artifact.id.clone());
            }
        }

        let ids: Vec<String> = schema.artifacts.iter().map(|a| a.id.clone()).collect();

        let apply_requires = schema.apply.as_ref().map_or_else(Vec::new, |a| a.requires.clone());

        let graph = Self {
            ids,
            generates,
            dependents,
            dependencies,
            apply_requires,
        };

        graph.verify_acyclic()?;
        Ok(graph)
    }

    /// Return artifact IDs in topological (dependency-first) order via Kahn's algorithm.
    #[must_use]
    pub fn build_order(&self) -> Vec<&str> {
        let mut in_degree: HashMap<&str, usize> = HashMap::new();
        for id in &self.ids {
            in_degree.insert(id, self.dependencies.get(id.as_str()).map_or(0, Vec::len));
        }

        let mut queue: VecDeque<&str> =
            in_degree.iter().filter(|(_, deg)| **deg == 0).map(|(&id, _)| id).collect();

        let mut order = Vec::with_capacity(self.ids.len());

        while let Some(id) = queue.pop_front() {
            order.push(id);
            if let Some(deps) = self.dependents.get(id) {
                for dep in deps {
                    if let Some(deg) = in_degree.get_mut(dep.as_str()) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dep);
                        }
                    }
                }
            }
        }

        order
    }

    /// Check artifact completion state by inspecting the filesystem under `change_dir`.
    #[must_use]
    pub fn completion(&self, change_dir: &Path) -> CompletionState {
        let completed: HashSet<String> =
            self.ids.iter().filter(|id| self.artifact_exists(id, change_dir)).cloned().collect();

        let mut ready = Vec::new();
        let mut blocked = Vec::new();

        for id in &self.ids {
            if completed.contains(id) {
                continue;
            }
            let deps = self.dependencies.get(id).cloned().unwrap_or_default();
            if deps.iter().all(|d| completed.contains(d)) {
                ready.push(id.clone());
            } else {
                let missing: Vec<String> =
                    deps.into_iter().filter(|d| !completed.contains(d)).collect();
                blocked.push(BlockedArtifact {
                    id: id.clone(),
                    waiting_on: missing,
                });
            }
        }

        let apply_ready = self.apply_requires.iter().all(|r| completed.contains(r));
        let task_progress = Self::parse_task_progress(change_dir);

        CompletionState {
            completed: completed.into_iter().collect(),
            ready,
            blocked,
            apply_ready,
            task_progress,
        }
    }

    /// All artifact IDs in schema order.
    #[must_use]
    pub fn artifact_ids(&self) -> &[String] {
        &self.ids
    }

    /// The `generates` pattern for a given artifact.
    #[must_use]
    pub fn generates(&self, id: &str) -> Option<&str> {
        self.generates.get(id).map(String::as_str)
    }

    /// Direct dependencies for a given artifact.
    #[must_use]
    pub fn dependencies_of(&self, id: &str) -> &[String] {
        self.dependencies.get(id).map_or(&[] as &[String], Vec::as_slice)
    }

    fn artifact_exists(&self, id: &str, change_dir: &Path) -> bool {
        let Some(pattern) = self.generates.get(id) else {
            return false;
        };

        if pattern.contains("**") {
            let specs_dir = change_dir.join("specs");
            specs_dir.is_dir() && dir_has_md_files(&specs_dir)
        } else {
            change_dir.join(pattern).is_file()
        }
    }

    fn parse_task_progress(change_dir: &Path) -> Option<TaskProgress> {
        let tasks_path = change_dir.join("tasks.md");
        let content = std::fs::read_to_string(tasks_path).ok()?;

        let mut total = 0u32;
        let mut done = 0u32;
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("- [ ] ") {
                total += 1;
            } else if trimmed.starts_with("- [x] ") || trimmed.starts_with("- [X] ") {
                total += 1;
                done += 1;
            }
        }

        Some(TaskProgress { total, done })
    }

    fn verify_acyclic(&self) -> Result<()> {
        let order = self.build_order();
        if order.len() != self.ids.len() {
            bail!("artifact graph contains a cycle");
        }
        Ok(())
    }
}

/// Recursively check whether a directory contains any `.md` files.
fn dir_has_md_files(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            return true;
        }
        if path.is_dir() && dir_has_md_files(&path) {
            return true;
        }
    }
    false
}

/// Artifact completion snapshot for a change directory.
#[derive(Debug, Clone, Serialize)]
pub struct CompletionState {
    /// Artifact IDs whose generated files exist on disk.
    pub completed: Vec<String>,
    /// Artifact IDs whose dependencies are met but files are missing.
    pub ready: Vec<String>,
    /// Artifact IDs still waiting on unfinished dependencies.
    pub blocked: Vec<BlockedArtifact>,
    /// Whether all apply-phase prerequisites are satisfied.
    pub apply_ready: bool,
    /// Checkbox progress from `tasks.md`, if present.
    pub task_progress: Option<TaskProgress>,
}

/// An artifact that cannot proceed until its dependencies are finished.
#[derive(Debug, Clone, Serialize)]
pub struct BlockedArtifact {
    /// The blocked artifact's ID.
    pub id: String,
    /// IDs of unfinished dependencies.
    pub waiting_on: Vec<String>,
}

/// Checkbox progress extracted from `tasks.md`.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct TaskProgress {
    /// Total number of checkbox items.
    pub total: u32,
    /// Number of checked items.
    pub done: u32,
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;
    use crate::core::schema::{ApplyConfig, Artifact, Schema};

    fn test_schema() -> Schema {
        Schema {
            name: "test".to_string(),
            version: 1,
            description: "test schema".to_string(),
            artifacts: vec![
                Artifact {
                    id: "proposal".to_string(),
                    generates: "proposal.md".to_string(),
                    description: String::new(),
                    template: "proposal.md".to_string(),
                    instruction: String::new(),
                    requires: vec![],
                },
                Artifact {
                    id: "specs".to_string(),
                    generates: "specs/**/*.md".to_string(),
                    description: String::new(),
                    template: "spec.md".to_string(),
                    instruction: String::new(),
                    requires: vec!["proposal".to_string()],
                },
                Artifact {
                    id: "design".to_string(),
                    generates: "design.md".to_string(),
                    description: String::new(),
                    template: "design.md".to_string(),
                    instruction: String::new(),
                    requires: vec!["proposal".to_string()],
                },
                Artifact {
                    id: "tasks".to_string(),
                    generates: "tasks.md".to_string(),
                    description: String::new(),
                    template: "tasks.md".to_string(),
                    instruction: String::new(),
                    requires: vec!["specs".to_string(), "design".to_string()],
                },
            ],
            apply: Some(ApplyConfig {
                requires: vec!["tasks".to_string()],
                tracks: Some("tasks.md".to_string()),
                instruction: String::new(),
            }),
        }
    }

    #[test]
    fn topological_order_respects_dependencies() {
        let schema = test_schema();
        let graph = ArtifactGraph::from_schema(&schema).unwrap();
        let order = graph.build_order();

        let pos = |id: &str| order.iter().position(|&x| x == id).unwrap();
        assert!(pos("proposal") < pos("specs"));
        assert!(pos("proposal") < pos("design"));
        assert!(pos("specs") < pos("tasks"));
        assert!(pos("design") < pos("tasks"));
    }

    #[test]
    fn cycle_detection() {
        let schema = Schema {
            name: "cycle".to_string(),
            version: 1,
            description: String::new(),
            artifacts: vec![
                Artifact {
                    id: "a".to_string(),
                    generates: "a.md".to_string(),
                    description: String::new(),
                    template: "a.md".to_string(),
                    instruction: String::new(),
                    requires: vec!["b".to_string()],
                },
                Artifact {
                    id: "b".to_string(),
                    generates: "b.md".to_string(),
                    description: String::new(),
                    template: "b.md".to_string(),
                    instruction: String::new(),
                    requires: vec!["a".to_string()],
                },
            ],
            apply: None,
        };

        let result = ArtifactGraph::from_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("cycle"));
    }

    #[test]
    fn unknown_dependency_rejected() {
        let schema = Schema {
            name: "bad".to_string(),
            version: 1,
            description: String::new(),
            artifacts: vec![Artifact {
                id: "a".to_string(),
                generates: "a.md".to_string(),
                description: String::new(),
                template: "a.md".to_string(),
                instruction: String::new(),
                requires: vec!["nonexistent".to_string()],
            }],
            apply: None,
        };

        let result = ArtifactGraph::from_schema(&schema);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonexistent"));
    }

    #[test]
    fn completion_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let schema = test_schema();
        let graph = ArtifactGraph::from_schema(&schema).unwrap();
        let state = graph.completion(tmp.path());

        assert!(state.completed.is_empty());
        assert_eq!(state.ready, vec!["proposal"]);
        assert_eq!(state.blocked.len(), 3);
        assert!(!state.apply_ready);
    }

    #[test]
    fn completion_tracks_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("proposal.md"), "# Proposal").unwrap();
        fs::write(tmp.path().join("design.md"), "# Design").unwrap();
        let specs = tmp.path().join("specs").join("my-cap");
        fs::create_dir_all(&specs).unwrap();
        fs::write(specs.join("spec.md"), "# Spec").unwrap();

        let schema = test_schema();
        let graph = ArtifactGraph::from_schema(&schema).unwrap();
        let state = graph.completion(tmp.path());

        assert!(state.completed.contains(&"proposal".to_string()));
        assert!(state.completed.contains(&"specs".to_string()));
        assert!(state.completed.contains(&"design".to_string()));
        assert_eq!(state.ready, vec!["tasks"]);
        assert!(state.blocked.is_empty());
        assert!(!state.apply_ready);
    }

    #[test]
    fn completion_all_done() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("proposal.md"), "done").unwrap();
        fs::write(tmp.path().join("design.md"), "done").unwrap();
        fs::write(tmp.path().join("tasks.md"), "- [x] 1.1 Do it\n- [ ] 1.2 Next\n").unwrap();
        let specs = tmp.path().join("specs").join("cap");
        fs::create_dir_all(&specs).unwrap();
        fs::write(specs.join("spec.md"), "spec").unwrap();

        let schema = test_schema();
        let graph = ArtifactGraph::from_schema(&schema).unwrap();
        let state = graph.completion(tmp.path());

        assert_eq!(state.completed.len(), 4);
        assert!(state.ready.is_empty());
        assert!(state.blocked.is_empty());
        assert!(state.apply_ready);

        let progress = state.task_progress.unwrap();
        assert_eq!(progress.total, 2);
        assert_eq!(progress.done, 1);
    }
}

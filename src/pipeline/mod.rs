mod graph;
mod grouping;

use std::collections::HashSet;
use std::path::Path;

use anyhow::{Result, bail};
use serde::Deserialize;

use crate::registry::Registry;

/// Change pipeline: targets and their dependencies for a single change.
#[derive(Debug, Deserialize)]
pub struct Pipeline {
    /// Deserialized for validation; not read at runtime yet.
    #[allow(dead_code)]
    pub change: String,
    pub targets: Vec<Target>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

/// A single target (service) in the pipeline.
#[derive(Debug, Clone, Deserialize)]
pub struct Target {
    pub id: String,
    pub specs: Vec<String>,
    pub repo: Option<String>,
    #[serde(rename = "crate")]
    pub crate_name: Option<String>,
    pub project_dir: Option<String>,
    pub branch: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
}

/// Known dependency types between targets.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DepType {
    /// Event schema dependency.
    EventSchema,
    /// HTTP API contract dependency.
    HttpApi,
    /// Shared types dependency.
    SharedTypes,
    /// Custom dependency type.
    #[serde(untagged)]
    Other(String),
}

/// Rich dependency metadata between targets.
/// `dep_type` and `contract` are deserialized for future contract validation.
#[derive(Debug, Clone, Deserialize)]
pub struct Dependency {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    #[allow(dead_code)]
    pub dep_type: DepType,
    #[allow(dead_code)]
    pub contract: Option<String>,
}

/// Targets grouped by their shared repo URL. One group = one branch + one PR.
#[derive(Debug)]
pub struct RepoGroup {
    pub repo: String,
    pub project_dir: String,
    pub domain: String,
    /// Explicit branch override shared by all targets in this group, if any.
    pub branch: Option<String>,
    pub targets: Vec<Target>,
    pub crates: Vec<String>,
    pub specs: Vec<String>,
}

impl RepoGroup {
    /// Derive the branch name for this group's change.
    /// Uses the validated branch override if set, otherwise `alc/<change>`.
    pub fn branch_name(&self, change: &str) -> String {
        self.branch
            .clone()
            .unwrap_or_else(|| format!("alc/{change}"))
    }

    /// Short label extracted from the repo URL (e.g. "train" from `git@github.com:org/train.git`).
    pub fn repo_label(&self) -> &str {
        crate::git::repo_name(&self.repo)
    }
}

impl Pipeline {
    /// Load pipeline from a TOML file.
    pub fn load(path: &Path) -> Result<Self> {
        crate::util::load_toml(path)
    }

    /// Collect all dependency edges from both `[[dependencies]]` and inline `depends_on`.
    pub(crate) fn all_edges(&self) -> Vec<(&str, &str)> {
        let mut edges: Vec<(&str, &str)> = self
            .dependencies
            .iter()
            .map(|d| (d.from.as_str(), d.to.as_str()))
            .collect();

        for target in &self.targets {
            for dep in &target.depends_on {
                edges.push((dep.as_str(), target.id.as_str()));
            }
        }
        edges
    }

    /// Validate pipeline integrity against registry and on-disk specs.
    pub fn validate(&self, registry: &Registry, change_dir: &Path) -> Result<()> {
        if self.targets.is_empty() {
            bail!("pipeline has no targets");
        }

        let mut target_ids = HashSet::new();
        for target in &self.targets {
            if !target_ids.insert(target.id.as_str()) {
                bail!("duplicate target id in pipeline: '{}'", target.id);
            }
            if registry.find_by_id(&target.id).is_none() {
                bail!("pipeline target '{}' not found in registry.toml", target.id);
            }
            for spec in &target.specs {
                if !spec_exists(change_dir, spec) {
                    bail!("target '{}' references missing spec '{}'", target.id, spec);
                }
            }
        }

        for (from, to) in self.all_edges() {
            if from == to {
                bail!("self-dependency is not allowed for target '{from}'");
            }
            if !target_ids.contains(from) {
                bail!("dependency references unknown 'from' target '{from}'");
            }
            if !target_ids.contains(to) {
                bail!("dependency references unknown 'to' target '{to}'");
            }
        }

        Ok(())
    }
}

fn spec_exists(change_dir: &Path, spec: &str) -> bool {
    let specs_root = change_dir.join("specs");
    let direct = specs_root.join(spec);
    let nested = specs_root.join(spec).join("spec.md");
    let md = specs_root.join(format!("{spec}.md"));
    direct.is_file() || nested.is_file() || md.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_rejects_duplicate_target_ids() {
        let toml_str = r#"
change = "test"
[[targets]]
id = "a"
specs = []
[[targets]]
id = "a"
specs = []
"#;
        let p: Pipeline = toml::from_str(toml_str).expect("parsing pipeline");
        let reg: Registry = toml::from_str(
            r#"
[[services]]
id = "a"
repo = "git@github.com:org/repo.git"
project_dir = "."
crate = "a"
domain = "d"
capabilities = []
"#,
        )
        .expect("parsing registry");
        let tmp = std::env::temp_dir().join(format!("opsx-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(tmp.join("specs"));
        assert!(p.validate(&reg, &tmp).is_err());
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn malformed_pipeline_toml_gives_error() {
        let bad_toml = "this is not valid toml [[[";
        let result: Result<Pipeline, _> = toml::from_str(bad_toml);
        assert!(result.is_err());
    }

    #[test]
    fn pipeline_missing_change_field() {
        let toml_str = r#"
[[targets]]
id = "a"
specs = []
"#;
        let result: Result<Pipeline, _> = toml::from_str(toml_str);
        assert!(result.is_err());
    }

    #[test]
    fn validation_rejects_empty_pipeline() {
        let toml_str = "change = \"test\"\ntargets = []";
        let p: Pipeline = toml::from_str(toml_str).unwrap();
        let reg: Registry = toml::from_str(
            r#"[[services]]
id = "x"
repo = "r"
project_dir = "."
crate = "x"
domain = "d"
capabilities = []
"#,
        )
        .unwrap();
        let tmp = std::env::temp_dir();
        assert!(p.validate(&reg, &tmp).unwrap_err().to_string().contains("no targets"));
    }
}

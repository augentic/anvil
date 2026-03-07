use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::registry::Registry;

#[derive(Debug, Deserialize)]
pub struct Pipeline {
    pub change: String,
    pub targets: Vec<Target>,
    /// Optional rich metadata about cross-target dependencies.
    /// NOT used for ordering -- ordering comes solely from `depends_on` on each target.
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
}

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

/// Rich dependency metadata between targets (type, contract).
/// Informational only -- not used for execution ordering.
#[derive(Debug, Clone, Deserialize)]
pub struct Dependency {
    pub from: String,
    pub to: String,
    #[serde(rename = "type")]
    pub dep_type: String,
    pub contract: Option<String>,
}

/// Targets grouped by their shared repo URL. One group = one branch + one PR.
#[derive(Debug)]
pub struct RepoGroup {
    pub repo: String,
    pub project_dir: String,
    pub domain: String,
    pub targets: Vec<Target>,
    pub crates: Vec<String>,
    pub specs: Vec<String>,
}

impl Pipeline {
    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))
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

        for target in &self.targets {
            for dep in &target.depends_on {
                if dep == &target.id {
                    bail!("self-dependency is not allowed for target '{}'", target.id);
                }
                if !target_ids.contains(dep.as_str()) {
                    bail!(
                        "target '{}' depends_on unknown target '{dep}'",
                        target.id
                    );
                }
            }
        }

        for d in &self.dependencies {
            if !target_ids.contains(d.from.as_str()) {
                bail!(
                    "[[dependencies]] references unknown 'from' target '{}'",
                    d.from
                );
            }
            if !target_ids.contains(d.to.as_str()) {
                bail!(
                    "[[dependencies]] references unknown 'to' target '{}'",
                    d.to
                );
            }
        }

        Ok(())
    }

    /// Kahn's algorithm: returns targets in dependency order (upstream first).
    /// Ordering is driven solely by `depends_on` on each target.
    pub fn topological_sort(&self) -> Result<Vec<&Target>> {
        let target_ids: HashSet<&str> = self.targets.iter().map(|t| t.id.as_str()).collect();
        let mut in_degree: HashMap<&str, usize> = target_ids.iter().map(|id| (*id, 0)).collect();
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

        for target in &self.targets {
            for dep in &target.depends_on {
                if !target_ids.contains(dep.as_str()) {
                    bail!("depends_on references unknown target '{dep}'");
                }
                *in_degree.entry(target.id.as_str()).or_default() += 1;
                dependents
                    .entry(dep.as_str())
                    .or_default()
                    .push(target.id.as_str());
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order: Vec<&str> = Vec::with_capacity(self.targets.len());

        while let Some(id) = queue.pop_front() {
            order.push(id);
            if let Some(deps) = dependents.get(id) {
                for &dep in deps {
                    let deg = in_degree
                        .get_mut(dep)
                        .expect("in_degree populated for all target IDs");
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dep);
                    }
                }
            }
        }

        if order.len() != self.targets.len() {
            bail!("dependency cycle detected in pipeline targets");
        }

        let target_map: HashMap<&str, &Target> =
            self.targets.iter().map(|t| (t.id.as_str(), t)).collect();
        Ok(order.into_iter().map(|id| target_map[id]).collect())
    }

    /// Group pipeline targets by their repo URL (from the registry).
    pub fn group_by_repo(&self, registry: &Registry) -> Result<Vec<RepoGroup>> {
        let mut groups: HashMap<String, RepoGroup> = HashMap::new();

        for target in &self.targets {
            let svc = registry
                .find_by_id(&target.id)
                .with_context(|| format!("target '{}' not found in registry.toml", target.id))?;

            let repo = target.repo.as_deref().unwrap_or(&svc.repo);
            let project_dir = target.project_dir.as_deref().unwrap_or(&svc.project_dir);
            let crate_name = target.crate_name.as_deref().unwrap_or(&svc.crate_name);

            let group = groups.entry(repo.to_string()).or_insert_with(|| RepoGroup {
                repo: repo.to_string(),
                project_dir: project_dir.to_string(),
                domain: svc.domain.clone(),
                targets: Vec::new(),
                crates: Vec::new(),
                specs: Vec::new(),
            });

            group.targets.push(target.clone());
            group.crates.push(crate_name.to_string());
            group.specs.extend(target.specs.clone());
        }

        Ok(groups.into_values().collect())
    }

    /// Return target IDs that `id` depends on (must complete before `id`).
    pub fn upstream_of(&self, id: &str) -> Vec<&str> {
        self.targets
            .iter()
            .find(|t| t.id == id)
            .map(|t| t.depends_on.iter().map(String::as_str).collect())
            .unwrap_or_default()
    }

    /// Sort repo groups so that groups with upstream dependencies come first.
    /// A group must run before another if any target in the second group
    /// depends on a target in the first group.
    /// Sort repo groups so that groups with upstream dependencies come first.
    /// A group must run before another if any target in the second group
    /// depends on a target in the first group.
    pub fn groups_in_dependency_order(&self, registry: &Registry) -> Result<Vec<RepoGroup>> {
        let groups = self.group_by_repo(registry)?;
        if groups.len() <= 1 {
            return Ok(groups);
        }

        // Build target -> repo index from the pipeline + registry (not from groups)
        // to avoid borrowing groups.
        let mut target_to_repo: HashMap<&str, String> = HashMap::new();
        for target in &self.targets {
            let svc = registry
                .find_by_id(&target.id)
                .with_context(|| format!("target '{}' not in registry", target.id))?;
            let repo = target.repo.as_deref().unwrap_or(&svc.repo);
            target_to_repo.insert(target.id.as_str(), repo.to_string());
        }

        let repo_ids: Vec<String> = groups.iter().map(|g| g.repo.clone()).collect();
        let mut in_degree: HashMap<&str, usize> =
            repo_ids.iter().map(|r| (r.as_str(), 0)).collect();
        let mut dependents: HashMap<&str, HashSet<&str>> = HashMap::new();

        for target in &self.targets {
            let target_repo = &target_to_repo[target.id.as_str()];
            for dep in &target.depends_on {
                if let Some(dep_repo) = target_to_repo.get(dep.as_str())
                    && dep_repo != target_repo
                {
                    let target_repo_str = repo_ids
                        .iter()
                        .find(|r| r.as_str() == target_repo.as_str())
                        .expect("repo in list")
                        .as_str();
                    let dep_repo_str = repo_ids
                        .iter()
                        .find(|r| r.as_str() == dep_repo.as_str())
                        .expect("dep repo in list")
                        .as_str();
                    if dependents
                        .entry(dep_repo_str)
                        .or_default()
                        .insert(target_repo_str)
                    {
                        *in_degree.entry(target_repo_str).or_default() += 1;
                    }
                }
            }
        }

        let mut queue: VecDeque<&str> = in_degree
            .iter()
            .filter(|(_, deg)| **deg == 0)
            .map(|(&id, _)| id)
            .collect();

        let mut order: Vec<&str> = Vec::with_capacity(groups.len());
        while let Some(repo) = queue.pop_front() {
            order.push(repo);
            if let Some(deps) = dependents.get(repo) {
                for &dep_repo in deps {
                    let deg = in_degree
                        .get_mut(dep_repo)
                        .expect("in_degree populated for all repos");
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push_back(dep_repo);
                    }
                }
            }
        }

        if order.len() != groups.len() {
            bail!("dependency cycle detected between repo groups");
        }

        let mut group_map: HashMap<String, RepoGroup> =
            groups.into_iter().map(|g| (g.repo.clone(), g)).collect();
        Ok(order
            .into_iter()
            .filter_map(|repo| group_map.remove(repo))
            .collect())
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

    fn sample_pipeline() -> Pipeline {
        let toml_str = r#"
change = "r9k-http"

[[targets]]
id = "r9k-connector"
specs = ["r9k-xml-ingest"]

[[targets]]
id = "r9k-adapter"
specs = ["r9k-xml-to-smartrak-gtfs"]
depends_on = ["r9k-connector"]

[[dependencies]]
from = "r9k-connector"
to = "r9k-adapter"
type = "event-schema"
contract = "domains/train/shared-types.md#R9kEvent"
"#;
        toml::from_str(toml_str).expect("parsing sample pipeline")
    }

    #[test]
    fn topological_sort_respects_deps() {
        let p = sample_pipeline();
        let sorted = p.topological_sort().expect("topological sort");
        assert_eq!(sorted[0].id, "r9k-connector");
        assert_eq!(sorted[1].id, "r9k-adapter");
    }

    #[test]
    fn inline_depends_on_drives_sort() {
        let toml_str = r#"
change = "test"
[[targets]]
id = "a"
specs = []
[[targets]]
id = "b"
specs = []
depends_on = ["a"]
"#;
        let p: Pipeline = toml::from_str(toml_str).expect("parsing pipeline");
        let sorted = p.topological_sort().expect("topological sort");
        assert_eq!(sorted[0].id, "a");
        assert_eq!(sorted[1].id, "b");
    }

    #[test]
    fn detects_cycle() {
        let toml_str = r#"
change = "test"
[[targets]]
id = "a"
specs = []
depends_on = ["b"]
[[targets]]
id = "b"
specs = []
depends_on = ["a"]
"#;
        let p: Pipeline = toml::from_str(toml_str).expect("parsing pipeline");
        assert!(p.topological_sort().is_err());
    }

    #[test]
    fn upstream_of() {
        let p = sample_pipeline();
        let upstream = p.upstream_of("r9k-adapter");
        assert!(upstream.contains(&"r9k-connector"));
        assert!(p.upstream_of("r9k-connector").is_empty());
    }

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
        let tmp = std::env::temp_dir().join(format!("alc-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(tmp.join("specs"));
        assert!(p.validate(&reg, &tmp).is_err());
        let _ = std::fs::remove_dir_all(tmp);
    }

    #[test]
    fn group_by_repo_merges_targets_sharing_repo() {
        let p = sample_pipeline();
        let reg: Registry = toml::from_str(
            r#"
[[services]]
id = "r9k-connector"
repo = "git@github.com:wasm-replatform/train.git"
project_dir = "."
crate = "r9k-connector"
domain = "train"
capabilities = ["r9k-xml-ingest"]

[[services]]
id = "r9k-adapter"
repo = "git@github.com:wasm-replatform/train.git"
project_dir = "."
crate = "r9k-adapter"
domain = "train"
capabilities = ["r9k-xml-to-smartrak-gtfs"]
"#,
        )
        .expect("parsing registry");
        let groups = p.group_by_repo(&reg).expect("group by repo");
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].targets.len(), 2);
        assert!(groups[0].crates.contains(&String::from("r9k-connector")));
        assert!(groups[0].crates.contains(&String::from("r9k-adapter")));
    }

    #[test]
    fn dependencies_metadata_does_not_affect_ordering() {
        let toml_str = r#"
change = "test"
[[targets]]
id = "a"
specs = []
[[targets]]
id = "b"
specs = []

[[dependencies]]
from = "a"
to = "b"
type = "event-schema"
"#;
        let p: Pipeline = toml::from_str(toml_str).expect("parsing pipeline");
        let sorted = p.topological_sort().expect("topological sort");
        // Without depends_on, both are independent -- either order is valid
        assert_eq!(sorted.len(), 2);
        // The [[dependencies]] metadata is parsed but doesn't create ordering edges
        assert!(p.dependencies.len() == 1);
    }
}

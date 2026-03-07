use std::collections::{HashMap, HashSet, VecDeque};
use std::path::Path;

use anyhow::{Context, Result, bail};
use serde::Deserialize;

use crate::registry::Registry;

#[derive(Debug, Deserialize)]
pub struct Pipeline {
    pub change: String,
    pub targets: Vec<Target>,
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

/// Rich dependency metadata between targets.
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

impl RepoGroup {
    /// Derive the branch name for this group's change.
    /// Uses an explicit `branch` from the first target if set, otherwise `alc/<change>`.
    pub fn branch_name(&self, change: &str) -> String {
        self.targets
            .first()
            .and_then(|t| t.branch.as_deref())
            .map(String::from)
            .unwrap_or_else(|| format!("alc/{change}"))
    }

    /// Short label extracted from the repo URL (e.g. "train" from "git@github.com:org/train.git").
    pub fn repo_label(&self) -> String {
        self.repo
            .rsplit('/')
            .next()
            .unwrap_or("repo")
            .trim_end_matches(".git")
            .to_string()
    }
}

impl Pipeline {
    pub fn load(path: &Path) -> Result<Self> {
        let content =
            std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        toml::from_str(&content).with_context(|| format!("parsing {}", path.display()))
    }

    /// Collect all dependency edges from both `[[dependencies]]` and inline `depends_on`.
    fn all_edges(&self) -> Vec<(&str, &str)> {
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

    /// Kahn's algorithm: returns targets in dependency order (upstream first).
    pub fn topological_sort(&self) -> Result<Vec<&Target>> {
        let target_ids: HashSet<&str> = self.targets.iter().map(|t| t.id.as_str()).collect();
        let mut in_degree: HashMap<&str, usize> = target_ids.iter().map(|id| (*id, 0)).collect();
        let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();

        for (from, to) in self.all_edges() {
            if !target_ids.contains(from) {
                bail!("dependency references unknown target '{from}'");
            }
            if !target_ids.contains(to) {
                bail!("dependency references unknown target '{to}'");
            }
            *in_degree.entry(to).or_default() += 1;
            dependents.entry(from).or_default().push(to);
        }

        let mut queue: VecDeque<&str> =
            in_degree.iter().filter(|(_, deg)| **deg == 0).map(|(&id, _)| id).collect();

        let mut order: Vec<&str> = Vec::with_capacity(self.targets.len());

        while let Some(id) = queue.pop_front() {
            order.push(id);
            if let Some(deps) = dependents.get(id) {
                for &dep in deps {
                    let deg = in_degree.get_mut(dep).unwrap();
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

            if group.project_dir != project_dir {
                bail!(
                    "target '{}' has project_dir '{}' but repo group '{}' uses '{}'",
                    target.id, project_dir, repo, group.project_dir
                );
            }
            if group.domain != svc.domain {
                bail!(
                    "target '{}' has domain '{}' but repo group '{}' uses '{}'",
                    target.id, svc.domain, repo, group.domain
                );
            }

            group.targets.push(target.clone());
            group.crates.push(crate_name.to_string());
            group.specs.extend(target.specs.clone());
        }

        Ok(groups.into_values().collect())
    }

    /// Partition repo groups into dependency levels for parallel execution.
    /// Level 0 groups have no cross-group dependencies, level 1 depends only
    /// on level 0, etc. Groups within the same level can run concurrently.
    pub fn dependency_levels(&self, registry: &Registry) -> Result<Vec<Vec<RepoGroup>>> {
        let groups = self.group_by_repo(registry)?;
        if groups.is_empty() {
            return Ok(vec![]);
        }
        if groups.len() == 1 {
            return Ok(vec![groups]);
        }

        let mut target_to_repo: HashMap<&str, String> = HashMap::new();
        for target in &self.targets {
            let svc = registry
                .find_by_id(&target.id)
                .with_context(|| format!("target '{}' not in registry", target.id))?;
            let repo = target.repo.as_deref().unwrap_or(&svc.repo);
            target_to_repo.insert(target.id.as_str(), repo.to_string());
        }

        let repo_ids: Vec<String> = groups.iter().map(|g| g.repo.clone()).collect();

        let mut repo_deps: HashMap<&str, HashSet<&str>> = HashMap::new();
        for (from, to) in self.all_edges() {
            if let (Some(from_repo), Some(to_repo)) =
                (target_to_repo.get(from), target_to_repo.get(to))
            {
                if from_repo != to_repo {
                    let from_str = repo_ids.iter().find(|r| r.as_str() == from_repo.as_str()).expect("repo in list").as_str();
                    let to_str = repo_ids.iter().find(|r| r.as_str() == to_repo.as_str()).expect("repo in list").as_str();
                    repo_deps.entry(to_str).or_default().insert(from_str);
                }
            }
        }

        let mut repo_to_level: HashMap<&str, usize> = HashMap::new();
        for repo in &repo_ids {
            assign_level(repo.as_str(), &repo_deps, &mut repo_to_level);
        }

        let max_level = repo_to_level.values().copied().max().unwrap_or(0);
        let mut group_map: HashMap<String, RepoGroup> =
            groups.into_iter().map(|g| (g.repo.clone(), g)).collect();

        let mut levels: Vec<Vec<RepoGroup>> = Vec::with_capacity(max_level + 1);
        for level in 0..=max_level {
            let level_groups: Vec<RepoGroup> = repo_ids
                .iter()
                .filter(|r| repo_to_level.get(r.as_str()) == Some(&level))
                .filter_map(|r| group_map.remove(r.as_str()))
                .collect();
            if !level_groups.is_empty() {
                levels.push(level_groups);
            }
        }

        Ok(levels)
    }

    /// Return targets that `id` depends on (i.e., must complete before `id`).
    pub fn upstream_of(&self, id: &str) -> Vec<&str> {
        let mut upstream: Vec<&str> = self
            .dependencies
            .iter()
            .filter(|d| d.to == id)
            .map(|d| d.from.as_str())
            .collect();

        if let Some(target) = self.targets.iter().find(|t| t.id == id) {
            for dep in &target.depends_on {
                if !upstream.contains(&dep.as_str()) {
                    upstream.push(dep.as_str());
                }
            }
        }

        upstream
    }
}

fn assign_level<'a>(
    repo: &'a str,
    deps: &HashMap<&'a str, HashSet<&'a str>>,
    levels: &mut HashMap<&'a str, usize>,
) -> usize {
    if let Some(&lvl) = levels.get(repo) {
        return lvl;
    }
    let lvl = deps
        .get(repo)
        .map(|upstream| {
            upstream
                .iter()
                .map(|dep| assign_level(dep, deps, levels) + 1)
                .max()
                .unwrap_or(0)
        })
        .unwrap_or(0);
    levels.insert(repo, lvl);
    lvl
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
        toml::from_str(toml_str).unwrap()
    }

    #[test]
    fn topological_sort_respects_deps() {
        let p = sample_pipeline();
        let sorted = p.topological_sort().unwrap();
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
        let p: Pipeline = toml::from_str(toml_str).unwrap();
        let sorted = p.topological_sort().unwrap();
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
[[targets]]
id = "b"
specs = []
[[dependencies]]
from = "a"
to = "b"
type = "x"
[[dependencies]]
from = "b"
to = "a"
type = "x"
"#;
        let p: Pipeline = toml::from_str(toml_str).unwrap();
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
        let tmp = std::env::temp_dir().join(format!("opsx-test-{}", std::process::id()));
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
}

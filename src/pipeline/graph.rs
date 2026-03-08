use std::collections::{HashMap, HashSet, VecDeque};

use anyhow::{Context, Result, bail};

use super::{Pipeline, RepoGroup, Target};
use crate::registry::Registry;

impl Pipeline {
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
                    let deg = in_degree.get_mut(dep)
                        .with_context(|| format!("target '{dep}' missing from in_degree map"))?;
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
                && from_repo != to_repo
            {
                let from_str = repo_ids.iter().find(|r| r.as_str() == from_repo.as_str())
                    .with_context(|| format!("repo '{}' not found in groups", from_repo))?.as_str();
                let to_str = repo_ids.iter().find(|r| r.as_str() == to_repo.as_str())
                    .with_context(|| format!("repo '{}' not found in groups", to_repo))?.as_str();
                repo_deps.entry(to_str).or_default().insert(from_str);
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
}

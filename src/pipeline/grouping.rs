use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use super::{Pipeline, RepoGroup};
use crate::registry::Registry;

impl Pipeline {
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
                branch: target.branch.clone(),
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
            match (&group.branch, &target.branch) {
                (Some(existing), Some(incoming)) if existing != incoming => {
                    bail!(
                        "target '{}' has branch '{}' but repo group '{}' already uses branch '{}'",
                        target.id, incoming, repo, existing
                    );
                }
                (None, Some(b)) => group.branch = Some(b.clone()),
                _ => {}
            }

            group.targets.push(target.clone());
            group.crates.push(crate_name.to_string());
            group.specs.extend(target.specs.clone());
        }

        Ok(groups.into_values().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn group_by_repo_merges_targets_sharing_repo() {
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
        let p: Pipeline = toml::from_str(toml_str).unwrap();
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

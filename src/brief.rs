use std::path::Path;

use anyhow::{Context, Result};

use crate::pipeline::RepoGroup;
use crate::engine::{BriefChange, BriefSpecs, BriefTarget, BriefUpstream, ChangeBrief};

/// Build a change brief for a repo group.
pub fn generate(change_name: &str, group: &RepoGroup) -> ChangeBrief {
    ChangeBrief {
        change: BriefChange {
            name: change_name.to_string(),
        },
        target: BriefTarget {
            repo: group.repo.clone(),
            crates: group.crates.clone(),
            domain: group.domain.clone(),
        },
        specs: BriefSpecs {
            files: group.specs.iter().map(|s| format!("{s}/spec.md")).collect(),
        },
        upstream: BriefUpstream {
            design: "upstream/design.md".to_string(),
            tasks: "upstream/tasks.md".to_string(),
            pipeline: "upstream/pipeline.toml".to_string(),
        },
    }
}

/// Write a brief to disk as TOML.
pub fn write(brief: &ChangeBrief, dest: &Path) -> Result<()> {
    let content = toml::to_string_pretty(brief).context("serializing brief")?;
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(dest, content).with_context(|| format!("writing {}", dest.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::{RepoGroup, Target};

    #[test]
    fn generate_brief_includes_specs_and_target_context() {
        let group = RepoGroup {
            repo: String::from("git@github.com:org/train.git"),
            project_dir: String::from("."),
            domain: String::from("train"),
            targets: vec![Target {
                id: String::from("r9k-connector"),
                specs: vec![String::from("r9k-xml-ingest")],
                repo: None,
                crate_name: None,
                project_dir: None,
                branch: None,
                depends_on: vec![],
            }],
            crates: vec![String::from("r9k-connector")],
            specs: vec![String::from("r9k-xml-ingest")],
        };

        let brief = generate("r9k-http", &group);
        assert_eq!(brief.change.name, "r9k-http");
        assert_eq!(brief.target.domain, "train");
        assert_eq!(brief.target.crates, vec![String::from("r9k-connector")]);
        assert_eq!(brief.specs.files, vec![String::from("r9k-xml-ingest/spec.md")]);
    }
}

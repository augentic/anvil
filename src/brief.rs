use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::engine::opsx::OpsxEngine;
use crate::pipeline::RepoGroup;

/// Per-repo artefact summarising what the change means for this repo group.
#[derive(Debug, Serialize, Deserialize)]
pub struct ChangeBrief {
    pub change: BriefChange,
    pub target: BriefTarget,
    pub specs: BriefSpecs,
    pub upstream: BriefUpstream,
}

/// Change metadata (name) for the brief.
#[derive(Debug, Serialize, Deserialize)]
pub struct BriefChange {
    pub name: String,
}

/// Target repo and crates for the brief.
#[derive(Debug, Serialize, Deserialize)]
pub struct BriefTarget {
    pub repo: String,
    pub crates: Vec<String>,
    pub domain: String,
}

/// Spec file paths referenced by the brief.
#[derive(Debug, Serialize, Deserialize)]
pub struct BriefSpecs {
    pub files: Vec<String>,
}

/// Paths to upstream design, tasks, and pipeline files.
#[derive(Debug, Serialize, Deserialize)]
pub struct BriefUpstream {
    pub design: String,
    pub tasks: String,
    pub pipeline: String,
}

/// Build a change brief for a repo group, using engine-provided paths.
pub fn generate(change_name: &str, group: &RepoGroup, engine: &OpsxEngine) -> ChangeBrief {
    let paths = engine.upstream_paths();
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
            files: group
                .specs
                .iter()
                .map(|s| engine.spec_file_path(s))
                .collect(),
        },
        upstream: BriefUpstream {
            design: paths.design.to_string(),
            tasks: paths.tasks.to_string(),
            pipeline: paths.pipeline.to_string(),
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
    use crate::engine::opsx::OpsxEngine;
    use crate::pipeline::{RepoGroup, Target};

    #[test]
    fn generate_brief_includes_specs_and_target_context() {
        let engine = OpsxEngine;
        let group = RepoGroup {
            repo: String::from("git@github.com:org/train.git"),
            project_dir: String::from("."),
            domain: String::from("train"),
            branch: None,
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

        let brief = generate("r9k-http", &group, &engine);
        assert_eq!(brief.change.name, "r9k-http");
        assert_eq!(brief.target.domain, "train");
        assert_eq!(brief.target.crates, vec![String::from("r9k-connector")]);
        assert_eq!(
            brief.specs.files,
            vec![String::from("r9k-xml-ingest/spec.md")]
        );
        assert_eq!(brief.upstream.design, "upstream/design.md");
    }
}

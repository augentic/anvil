//! Init-time AGENTS.md fence generation. Heavy lifting lives in
//! submodules; this module owns the small set of helpers
//! (`render_document` plus IO/error-mapping shims) used by init.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

mod assemble;
mod generate;

pub use generate::for_init as generate_for_init;
use specify_error::{Error, Result};
use specify_workflow::agents::{fences, fingerprint, render};
#[cfg(test)]
use specify_workflow::config::Layout;

use crate::runtime::context::Ctx;

fn render_document(ctx: &Ctx) -> Result<(String, fingerprint::ContextFingerprint)> {
    let assembly = assemble::render_input(ctx)?;
    let aggregate = fingerprint::aggregate(env!("CARGO_PKG_VERSION"), assembly.inputs.clone());
    let generated = render::render_document_with_fingerprint(&assembly.input, &aggregate);
    let fenced = fences::parse_document(generated.as_bytes())
        .map_err(|err| Error::Diag {
            code: "context-generated-document-fence-error",
            detail: err.to_string(),
        })?
        .ok_or_else(|| Error::Diag {
            code: "context-generated-document-missing-fences",
            detail: "generated AGENTS.md content must contain a Specify context fence".to_string(),
        })?;
    let context_fingerprint =
        fingerprint::for_context(env!("CARGO_PKG_VERSION"), assembly.inputs, fenced.body());
    Ok((generated, context_fingerprint))
}

fn read_optional(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(Error::Io(err)),
    }
}

fn error_from_fence(err: fences::FenceError) -> Error {
    match err {
        fences::FenceError::ExistingUnfencedAgentsMd => Error::Diag {
            code: "context-existing-unfenced-agents-md",
            detail: "AGENTS.md exists without Specify fences".to_string(),
        },
        other => Error::Diag {
            code: "context-fence-error",
            detail: other.to_string(),
        },
    }
}

fn context_lock_path(ctx: &Ctx) -> PathBuf {
    ctx.layout().specify_dir().join("context.lock")
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use specify_workflow::config::ProjectConfig;
    use specify_workflow::registry::Registry;
    use tempfile::tempdir;

    use super::*;
    use crate::output::Format;

    fn write_minimal_adapter(project_dir: &Path) {
        let adapter_dir = project_dir.join("adapters").join("targets").join("mini");
        let briefs_dir = adapter_dir.join("briefs");
        fs::create_dir_all(&briefs_dir).expect("create adapter dirs");
        fs::write(
            adapter_dir.join("adapter.yaml"),
            "name: mini\nversion: 1.0.0\naxis: target\nexecution: agent\nbriefs:\n  shape: briefs/shape.md\n  build: briefs/build.md\n  merge: briefs/merge.md\ndescription: Mini adapter\n",
        )
        .expect("write adapter");
        for op in ["shape", "build", "merge"] {
            fs::write(
                briefs_dir.join(format!("{op}.md")),
                format!("---\nid: {op}\ndescription: {op} brief\n---\n"),
            )
            .expect("write brief");
        }
    }

    fn sample_config() -> ProjectConfig {
        let mut rules = BTreeMap::new();
        rules.insert("proposal".to_string(), "rules/proposal.md".to_string());
        ProjectConfig {
            name: "demo".to_string(),
            description: Some("demo domain".to_string()),
            adapter: Some("mini".to_string()),
            specify_version: None,
            rules,
            tools: Vec::new(),
            platforms: Vec::new(),
            workspace: false,
        }
    }

    #[test]
    fn assemble_render_input_sorted_metadata() {
        let tmp = tempdir().expect("tempdir");
        write_minimal_adapter(tmp.path());
        let slices_dir = Layout::new(tmp.path()).slices_dir();
        fs::create_dir_all(slices_dir.join("zeta")).expect("create zeta");
        fs::create_dir_all(slices_dir.join("alpha")).expect("create alpha");
        fs::write(slices_dir.join("zeta").join("metadata.yaml"), "not parsed").expect("zeta meta");
        fs::write(slices_dir.join("alpha").join("metadata.yaml"), "also not parsed")
            .expect("alpha meta");
        fs::write(
            Registry::path(tmp.path()),
            "version: 1\nprojects:\n  - name: zeta\n    url: ../zeta\n    adapter: mini@1.0.0\n    description: Zeta service\n  - name: alpha\n    url: ../alpha\n    adapter: mini@1.0.0\n    description: Alpha service\n",
        )
        .expect("write registry");
        let cfg_path = Layout::new(tmp.path()).config_path();
        fs::create_dir_all(cfg_path.parent().expect("config parent")).expect("create .specify");
        fs::write(&cfg_path, "name: demo\nadapter: mini\nrules:\n  proposal: rules/proposal.md\n")
            .expect("write project config");
        let ctx = Ctx {
            format: Format::Text,
            project_dir: tmp.path().to_path_buf(),
            config: sample_config(),
            plan_dir: None,
        };

        let assembly = assemble::render_input(&ctx).expect("assemble render input");
        let input = &assembly.input;

        assert_eq!(input.active_slices, vec!["alpha".to_string(), "zeta".to_string()]);
        assert_eq!(
            input.dependencies.iter().map(|peer| peer.name.as_str()).collect::<Vec<_>>(),
            vec!["alpha", "zeta"]
        );
        assert_eq!(input.adapter.as_ref().map(|adapter| adapter.name.as_str()), Some("mini"));
        assert_eq!(
            input.rule_overrides,
            vec![render::Rule {
                brief_id: "proposal".to_string(),
                path: ".specify/rules/proposal.md".to_string(),
            }]
        );
        assert_eq!(
            assembly.inputs.iter().map(|input| input.path.as_str()).collect::<Vec<_>>(),
            vec![
                ".specify/project.yaml",
                ".specify/slices/alpha/metadata.yaml",
                ".specify/slices/zeta/metadata.yaml",
                "adapters/targets/mini/adapter.yaml",
                "adapters/targets/mini/briefs/build.md",
                "adapters/targets/mini/briefs/merge.md",
                "adapters/targets/mini/briefs/shape.md",
                "registry.yaml",
            ]
        );
    }

    #[test]
    fn render_input_skips_for_workspaces() {
        let tmp = tempdir().expect("tempdir");
        let cfg_path = Layout::new(tmp.path()).config_path();
        fs::create_dir_all(cfg_path.parent().expect("config parent")).expect("create .specify");
        fs::write(&cfg_path, "name: platform\nworkspace: true\n").expect("write project config");
        let ctx = Ctx {
            format: Format::Text,
            project_dir: tmp.path().to_path_buf(),
            config: ProjectConfig {
                name: "platform".to_string(),
                description: None,
                adapter: None,
                specify_version: None,
                rules: BTreeMap::new(),
                tools: Vec::new(),
                platforms: Vec::new(),
                workspace: true,
            },
            plan_dir: None,
        };

        let assembly = assemble::render_input(&ctx).expect("workspace assembly");
        let input = &assembly.input;

        assert!(input.is_workspace);
        assert!(input.adapter.is_none());
        assert!(input.dependencies.is_empty());
        assert_eq!(
            assembly.inputs.iter().map(|input| input.path.as_str()).collect::<Vec<_>>(),
            vec![".specify/project.yaml"]
        );
    }
}

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use crate::init::{InitOptions, fixed_now, init};

fn workspace_opts<'a>(project_dir: &'a Path, name: &'a str) -> InitOptions<'a> {
    InitOptions {
        project_dir,
        adapter: None,
        name: Some(name),
        description: None,
        workspace: true,
        include_framework: false,
        platforms: None,
        upgrade: false,
    }
}

mod init {
    use super::*;

    // The canonical on-disk shape (project.yaml / registry.yaml contents,
    // absent phase-pipeline dirs) is pinned by the binary test
    // `tests/init/base.rs::workspace_writes_canonical_shape`; this module
    // keeps only the kernel behavior edges.

    #[test]
    fn refuses_existing_specify_dir() {
        let tmp = tempdir().unwrap();
        fs::create_dir_all(tmp.path().join(".specify")).unwrap();
        fs::write(tmp.path().join(".specify/project.yaml"), "name: existing\nadapter: omnia\n")
            .unwrap();

        let err = init(workspace_opts(tmp.path(), "platform-workspace"), fixed_now())
            .expect_err("must refuse over existing dir");
        match err {
            specify_error::Error::Diag { code, detail } => {
                assert_eq!(code, "workspace-init-specify-dir-exists");
                assert!(
                    detail.contains("refusing to scaffold"),
                    "diagnostic should explain the refusal, got: {detail}"
                );
                assert!(
                    detail.contains(".specify"),
                    "diagnostic should mention .specify, got: {detail}"
                );
            }
            other => panic!("wrong error variant: {other:?}"),
        }
        let on_disk = fs::read_to_string(tmp.path().join(".specify/project.yaml")).unwrap();
        assert_eq!(on_disk, "name: existing\nadapter: omnia\n");
    }

    #[test]
    fn wasm_pkg_config() {
        let tmp = tempdir().unwrap();
        let result = init(workspace_opts(tmp.path(), "platform-workspace"), fixed_now())
            .expect("workspace init ok");

        assert!(result.wasm_pkg_config_written, "fresh workspace init must write the file");
        let path = tmp.path().join(".specify/wasm-pkg.toml");
        assert!(path.is_file(), "wasm-pkg.toml must exist after workspace init");
        let contents = fs::read_to_string(&path).expect("read wasm-pkg.toml");
        assert!(contents.contains("default_registry = \"augentic.io\""));
        assert!(contents.contains("specify = \"augentic.io\""));
    }

    #[test]
    fn rejects_non_kebab_name() {
        let tmp = tempdir().unwrap();
        let err =
            init(workspace_opts(tmp.path(), "BadName"), fixed_now()).expect_err("non-kebab name");
        match err {
            specify_error::Error::Diag { code, detail } => {
                assert_eq!(code, "workspace-init-name-not-kebab");
                assert!(detail.contains("kebab-case"), "diagnostic should cite the rule: {detail}");
                assert!(
                    detail.contains("BadName"),
                    "diagnostic should echo the bad name: {detail}"
                );
            }
            other => panic!("wrong error variant: {other:?}"),
        }
        assert!(!tmp.path().join(".specify").exists(), "no .specify on validation failure");
    }
}

//! End-to-end coverage of the pure project-scoped verbs through the
//! transport-neutral `Operation` layer: each test builds the wire input
//! DTO, invokes the operation against a tempdir-anchored provider, and
//! asserts the filesystem effects and the typed failure taxonomy.
//!
//! This is the always-on native home of the widened-verb coverage:
//! `registry {add,validate,remove}`, `archive prune`,
//! `init`, and `journal show`. The exit-code
//! projection over these failures lives in `crates/transport`
//! (`Exit::from(&error::Error)`) and is pinned there.

use std::fs;
use std::path::{Path, PathBuf};

use mock::invoke::run;
use mock::session::Session;

/// Stage a one-project `registry.yaml` at the project root.
fn stage_registry(root: &Path) {
    fs::write(
        root.join("registry.yaml"),
        "version: 1\nprojects:\n  - name: alpha\n    url: git@example.com:org/alpha.git\n",
    )
    .expect("stage registry.yaml");
}

mod registry {
    use super::*;

    #[tokio::test]
    async fn add_mints_file() {
        let project = Session::scripted("mock", Vec::new());
        let body = run::<project::registry::handlers::Add, _, _>(
            project.provider(),
            project::registry::handlers::AddInput {
                name: "alpha".into(),
                url: "git@example.com:org/alpha.git".into(),
                adapter: None,
                description: None,
            },
        )
        .await
        .expect("add succeeds");
        assert_eq!(body.added.name, "alpha");
        let registry =
            fs::read_to_string(project.root().join("registry.yaml")).expect("registry.yaml");
        assert!(registry.contains("name: alpha"), "the add landed:\n{registry}");
    }

    #[tokio::test]
    async fn duplicate_add_errors() {
        let project = Session::scripted("mock", Vec::new());
        stage_registry(project.root());
        let err = run::<project::registry::handlers::Add, _, _>(
            project.provider(),
            project::registry::handlers::AddInput {
                name: "alpha".into(),
                url: "git@example.com:org/alpha.git".into(),
                adapter: None,
                description: None,
            },
        )
        .await
        .expect_err("duplicate add fails with the typed diagnostic");
        assert!(
            matches!(err.core(), error::Error::Diag { .. }),
            "duplicate add is the typed diagnostic, got {err:?}"
        );
    }

    #[tokio::test]
    async fn staged_catalogue_validates() {
        let project = Session::scripted("mock", Vec::new());
        stage_registry(project.root());
        run::<project::registry::handlers::Validate, _, _>(
            project.provider(),
            project::registry::handlers::ValidateInput {},
        )
        .await
        .expect("staged catalogue validates");
    }

    /// Regression: `registry validate` is documented to run before
    /// `emery init` — it anchors on the invocation root and must not
    /// demand `.emery/project.yaml`.
    #[tokio::test]
    async fn validate_pre_init() {
        let project = Session::bare(Vec::new());
        stage_registry(project.root());
        run::<project::registry::handlers::Validate, _, _>(
            project.provider(),
            project::registry::handlers::ValidateInput {},
        )
        .await
        .expect("pre-init registry validate succeeds");
    }

    #[tokio::test]
    async fn remove_drops_entry() {
        let project = Session::scripted("mock", Vec::new());
        stage_registry(project.root());
        run::<project::registry::handlers::Remove, _, _>(
            project.provider(),
            project::registry::handlers::RemoveInput { name: "alpha".into() },
        )
        .await
        .expect("remove succeeds");
        let registry =
            fs::read_to_string(project.root().join("registry.yaml")).expect("registry.yaml");
        assert!(!registry.contains("name: alpha"), "the remove landed:\n{registry}");
    }
}

mod archive {
    use super::*;

    /// Stage two dated archive entries under `.emery/archive/`.
    fn stage(root: &Path) -> PathBuf {
        let archive = root.join(".emery/archive");
        for name in ["2020-01-01-old", "2026-01-01-new"] {
            fs::create_dir_all(archive.join(name)).expect("stage archive entry");
            fs::write(archive.join(name).join("spec.md"), "# archived\n").expect("stage file");
        }
        archive
    }

    #[tokio::test]
    async fn prune_keeps_newest() {
        let project = Session::scripted("mock", Vec::new());
        let archive = stage(project.root());
        let body = run::<slice::handlers::Prune, _, _>(
            project.provider(),
            slice::handlers::PruneInput {
                keep: Some(1),
                older_than: None,
                dry_run: false,
            },
        )
        .await
        .expect("prune succeeds");
        assert_eq!(body.pruned, ["2020-01-01-old"]);
        assert!(!archive.join("2020-01-01-old").exists(), "the stale entry is pruned");
        assert!(archive.join("2026-01-01-new").exists(), "the newest entry survives --keep 1");
    }

    #[tokio::test]
    async fn prune_requires_bound() {
        let project = Session::scripted("mock", Vec::new());
        stage(project.root());
        let err = run::<slice::handlers::Prune, _, _>(
            project.provider(),
            slice::handlers::PruneInput {
                keep: None,
                older_than: None,
                dry_run: false,
            },
        )
        .await
        .expect_err("a bound-less prune refuses");
        assert!(
            matches!(err.core(), error::Error::Argument { .. }),
            "a bound-less prune refuses with the argument error, got {err:?}"
        );
    }
}

mod init {
    use super::*;

    #[tokio::test]
    async fn adapter_required() {
        let project = Session::bare(Vec::new());
        let err = run::<project::init::handlers::Init, _, _>(
            project.provider(),
            project::init::handlers::InitInput::default(),
        )
        .await
        .expect_err("a flag-less init fails typed");
        assert!(
            matches!(
                err.core(),
                error::Error::Validation { code, .. } if code == "init-adapter-required"
            ),
            "a missing adapter fails with init-adapter-required, got {err:?}"
        );
    }

    #[tokio::test]
    async fn reentry_and_upgrade() {
        let project = Session::bare(Vec::new());
        let input = || project::init::handlers::InitInput {
            name: Some("demo-workspace".into()),
            workspace: true,
            ..Default::default()
        };
        run::<project::init::handlers::Init, _, _>(project.provider(), input())
            .await
            .expect("workspace scaffold succeeds");

        // Plain re-run: a no-op that routes to `--upgrade`.
        let body = run::<project::init::handlers::Init, _, _>(project.provider(), input())
            .await
            .expect("init re-entry exits 0");
        assert_eq!(body.mode, project::init::handlers::InitMode::AlreadyInitialized);
        assert_eq!(body.adapter_name, "workspace");

        // The documented re-entry command succeeds over the project.
        let body = run::<project::init::handlers::Init, _, _>(
            project.provider(),
            project::init::handlers::InitInput {
                upgrade: true,
                ..Default::default()
            },
        )
        .await
        .expect("init --upgrade succeeds");
        assert_eq!(body.mode, project::init::handlers::InitMode::Upgraded);
        assert_eq!(body.emery_version, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn workspace_mode() {
        let project = Session::bare(Vec::new());
        let body = run::<project::init::handlers::Init, _, _>(
            project.provider(),
            project::init::handlers::InitInput {
                adapter: None,
                name: Some("demo-workspace".into()),
                description: None,
                workspace: true,
                platforms: None,
                upgrade: false,
            },
        )
        .await
        .expect("workspace scaffold succeeds");
        assert_eq!(body.adapter_name, "workspace");
        let config =
            fs::read_to_string(project.root().join(".emery/project.yaml")).expect("project.yaml");
        assert!(config.contains("workspace: true"), "workspace mode is recorded:\n{config}");
        assert!(config.contains("name: demo-workspace"), "the name override lands:\n{config}");
        assert!(
            project.root().join("registry.yaml").is_file(),
            "workspace init mints registry.yaml"
        );

        assert!(body.context_generated, "workspace init generates AGENTS.md context");
        let agents = fs::read_to_string(project.root().join("AGENTS.md")).expect("AGENTS.md");
        assert!(
            !agents.contains("## Runtime"),
            "workspace context omits the per-language sections:\n{agents}"
        );
        assert!(
            project.root().join(".emery/context.lock").is_file(),
            "the fingerprint sidecar lands beside the generated context"
        );
    }

    #[tokio::test]
    async fn existing_agents_md_preserved() {
        let project = Session::bare(Vec::new());
        let agents_path = project.root().join("AGENTS.md");
        fs::write(&agents_path, "# operator prose\n").expect("stage operator AGENTS.md");

        let body = run::<project::init::handlers::Init, _, _>(
            project.provider(),
            project::init::handlers::InitInput {
                adapter: None,
                name: Some("demo-workspace".into()),
                description: None,
                workspace: true,
                platforms: None,
                upgrade: false,
            },
        )
        .await
        .expect("scaffold succeeds");

        assert!(!body.context_generated, "an existing AGENTS.md suppresses generation");
        assert!(body.context_skipped);
        assert_eq!(body.context_skip_reason, Some("existing-agents-md"));
        assert_eq!(
            fs::read_to_string(&agents_path).expect("AGENTS.md"),
            "# operator prose\n",
            "the operator file is preserved byte-for-byte"
        );
        assert!(
            !project.root().join(".emery/context.lock").exists(),
            "no fingerprint sidecar is written when generation is skipped"
        );
    }
}

mod journal {
    use super::*;

    /// Stage one taxonomy event through the real appender so the
    /// fixture matches production per-actor wiring.
    fn stage_event(root: &Path) {
        let event = project::journal::Event::new(
            jiff::Timestamp::from_second(1_700_000_000).expect("valid timestamp"),
            project::journal::EventKind::SliceBuildStarted {
                slice_name: "billing".into(),
            },
        );
        project::journal::append_one(project::config::Layout::new(root), &event)
            .expect("stage journal");
    }

    #[tokio::test]
    async fn show_reads_filtered() {
        let project = Session::scripted("mock", Vec::new());
        stage_event(project.root());
        let matched = run::<project::journal::handlers::Show, _, _>(
            project.provider(),
            project::journal::handlers::ShowInput {
                filter: Some("slice.build".into()),
                limit: None,
            },
        )
        .await
        .expect("show succeeds");
        assert_eq!(matched.count, 1, "the staged event matches its prefix");
        let unmatched = run::<project::journal::handlers::Show, _, _>(
            project.provider(),
            project::journal::handlers::ShowInput {
                filter: Some("plan.".into()),
                limit: None,
            },
        )
        .await
        .expect("a filter with no matches still succeeds");
        assert_eq!(unmatched.count, 0, "no plan events were staged");
        assert!(
            project
                .root()
                .join(".emery/events")
                .join(format!("{}.jsonl", project::journal::DEFAULT_ACTOR))
                .is_file(),
            "the per-actor journal persists"
        );
        assert!(
            !project.root().join(".emery/journal.jsonl").exists(),
            "single-file journal is retired"
        );
    }
}

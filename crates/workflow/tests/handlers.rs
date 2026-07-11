//! End-to-end coverage of the pure project-scoped verbs through the
//! transport-neutral `Operation` layer: each test builds the wire input
//! DTO, invokes the operation against a tempdir-anchored provider, and
//! asserts the filesystem effects and the typed failure taxonomy.
//!
//! This is the always-on native home of the widened-verb coverage:
//! `registry {add,validate,remove}`, `archive prune`,
//! `init --scaffold-only`, and `journal {emit,show}`. The exit-code
//! projection over these failures lives in `crates/transport`
//! (`Exit::from(&error::Error)`) and is pinned there.

use std::fs;
use std::path::{Path, PathBuf};

mod common;

use common::{Project, run};

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
    async fn add_mints_registry() {
        let project = Project::initialised();
        let body = run::<workflow::registry::handlers::Add, _>(
            &project,
            workflow::registry::handlers::AddInput {
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
            fs::read_to_string(project.root.join("registry.yaml")).expect("registry.yaml");
        assert!(registry.contains("name: alpha"), "the add landed:\n{registry}");
    }

    #[tokio::test]
    async fn duplicate_add_fails() {
        let project = Project::initialised();
        stage_registry(&project.root);
        let err = run::<workflow::registry::handlers::Add, _>(
            &project,
            workflow::registry::handlers::AddInput {
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
    async fn validate_staged_catalogue() {
        let project = Project::initialised();
        stage_registry(&project.root);
        run::<workflow::registry::handlers::Validate, _>(
            &project,
            workflow::registry::handlers::ValidateInput {},
        )
        .await
        .expect("staged catalogue validates");
    }

    /// Regression: `registry validate` is documented to run before
    /// `specify init` — it anchors on the invocation root and must not
    /// demand `.specify/project.yaml`.
    #[tokio::test]
    async fn validate_runs_pre_init() {
        let project = Project::bare();
        stage_registry(&project.root);
        run::<workflow::registry::handlers::Validate, _>(
            &project,
            workflow::registry::handlers::ValidateInput {},
        )
        .await
        .expect("pre-init registry validate succeeds");
    }

    #[tokio::test]
    async fn remove_drops_entry() {
        let project = Project::initialised();
        stage_registry(&project.root);
        run::<workflow::registry::handlers::Remove, _>(
            &project,
            workflow::registry::handlers::RemoveInput { name: "alpha".into() },
        )
        .await
        .expect("remove succeeds");
        let registry =
            fs::read_to_string(project.root.join("registry.yaml")).expect("registry.yaml");
        assert!(!registry.contains("name: alpha"), "the remove landed:\n{registry}");
    }
}

mod archive {
    use super::*;

    /// Stage two dated archive entries under `.specify/archive/`.
    fn stage(root: &Path) -> PathBuf {
        let archive = root.join(".specify/archive");
        for name in ["2020-01-01-old", "2026-01-01-new"] {
            fs::create_dir_all(archive.join(name)).expect("stage archive entry");
            fs::write(archive.join(name).join("spec.md"), "# archived\n").expect("stage file");
        }
        archive
    }

    #[tokio::test]
    async fn prune_keeps_newest() {
        let project = Project::initialised();
        let archive = stage(&project.root);
        let body = run::<workflow::slice::handlers::Prune, _>(
            &project,
            workflow::slice::handlers::PruneInput {
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
        let project = Project::initialised();
        stage(&project.root);
        let err = run::<workflow::slice::handlers::Prune, _>(
            &project,
            workflow::slice::handlers::PruneInput {
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

mod scaffold {
    use super::*;

    #[tokio::test]
    async fn workspace_mode() {
        let project = Project::bare();
        let body = run::<workflow::init::handlers::Scaffold, _>(
            &project,
            workflow::init::handlers::ScaffoldInput {
                adapter: None,
                name: Some("demo-workspace".into()),
                description: None,
                workspace: true,
                platforms: None,
            },
        )
        .await
        .expect("workspace scaffold succeeds");
        assert_eq!(body.adapter_name, "workspace");
        let config =
            fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
        assert!(config.contains("workspace: true"), "workspace mode is recorded:\n{config}");
        assert!(config.contains("name: demo-workspace"), "the name override lands:\n{config}");
        assert!(project.root.join("registry.yaml").is_file(), "workspace init mints registry.yaml");

        assert!(body.context_generated, "workspace init generates AGENTS.md context");
        let agents = fs::read_to_string(project.root.join("AGENTS.md")).expect("AGENTS.md");
        assert!(
            !agents.contains("## Runtime"),
            "workspace context omits the per-language sections:\n{agents}"
        );
        assert!(
            project.root.join(".specify/context.lock").is_file(),
            "the fingerprint sidecar lands beside the generated context"
        );
    }

    #[tokio::test]
    async fn regular_mode() {
        let project = Project::bare();

        // Stage a fake `demo` component at the resolver's in-repo dev
        // probe path with a digest-valid describe sidecar beside it:
        // the resolver probes file presence and the sidecar supplies
        // the metadata, so no describe dispatch runs.
        let dev_dir = project.root.join("target/wasm32-wasip2/release");
        fs::create_dir_all(&dev_dir).expect("mkdir dev release dir");
        let component = dev_dir.join("demo.wasm");
        fs::write(&component, b"\0asm-component").expect("stage component");
        let digest = schema::cache::file_content_digest(&component);
        fs::write(
            dev_dir.join("demo.wasm.metadata.json"),
            format!("{{ \"digest\": \"{digest}\", \"metadata\": {{}} }}"),
        )
        .expect("stage describe sidecar");

        let body = run::<workflow::init::handlers::Scaffold, _>(
            &project,
            workflow::init::handlers::ScaffoldInput {
                adapter: Some("demo".into()),
                name: Some("demo-project".into()),
                description: None,
                workspace: false,
                platforms: None,
            },
        )
        .await
        .expect("scaffold succeeds");
        assert_eq!(body.adapter_name, "demo");
        let config =
            fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
        assert!(config.contains("adapter: demo"), "the adapter is recorded:\n{config}");
        assert!(project.root.join(".specify/slices").is_dir(), "the slice tree is scaffolded");

        assert!(body.context_generated, "init generates AGENTS.md context when absent");
        let agents = fs::read_to_string(project.root.join("AGENTS.md")).expect("AGENTS.md");
        assert!(
            agents.contains("<!-- specify:context begin"),
            "the generated context is fenced:\n{agents}"
        );
        assert!(
            agents.contains("adapter `demo`"),
            "the resolved adapter surfaces in Conventions:\n{agents}"
        );
        assert!(
            project.root.join(".specify/context.lock").is_file(),
            "the fingerprint sidecar lands beside the generated context"
        );
    }

    /// A provider whose resolver answers from memory — the
    /// native-harness (linked crates) shape: no component file exists
    /// anywhere on disk.
    #[derive(Clone)]
    struct Linked(Project);

    impl workflow::handler::Anchor for Linked {
        fn project_root(&self) -> &Path {
            &self.0.root
        }
    }

    impl workflow::adapter::Resolver for Linked {
        fn resolve_source(
            &self, adapter_ref: &workflow::adapter::AdapterRef, _project_dir: &Path,
        ) -> Result<workflow::adapter::ResolvedSource, error::Error> {
            workflow::adapter::resolver::source(
                adapter_ref,
                workflow::adapter::metadata::Metadata::default(),
                linked_origin(),
            )
        }

        fn resolve_target(
            &self, adapter_ref: &workflow::adapter::AdapterRef, _project_dir: &Path,
        ) -> Result<workflow::adapter::ResolvedTarget, error::Error> {
            workflow::adapter::resolver::target(
                adapter_ref,
                workflow::adapter::metadata::Metadata::default(),
                linked_origin(),
            )
        }
    }

    fn linked_origin() -> workflow::adapter::Origin {
        workflow::adapter::Origin {
            label: "native".to_string(),
            reference: "rust:target:demo".to_string(),
        }
    }

    #[tokio::test]
    async fn regular_mode_component_free() {
        // A bare adapter name is an identity, not a file: init defers
        // component resolution to the injected resolver, so no `.wasm`
        // artifact is staged anywhere for this test.
        let project = Project::bare();
        let body = omnia_guest::api::invoke::Invoker::new("specify", Linked(project.clone()))
            .invoke::<workflow::init::handlers::Scaffold>(
                omnia_guest::api::invocation::Invocation::new(
                    workflow::init::handlers::ScaffoldInput {
                        adapter: Some("demo".into()),
                        name: Some("demo-project".into()),
                        description: None,
                        workspace: false,
                        platforms: None,
                    },
                ),
            )
            .await
            .expect("component-free scaffold succeeds");
        assert_eq!(body.adapter_name, "demo");
        let config =
            fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
        assert!(config.contains("adapter: demo"), "the bare identity is recorded:\n{config}");
        assert!(
            !body.cache_present,
            "no component is mirrored into the project cache for a bare name"
        );
    }

    #[tokio::test]
    async fn existing_agents_md_preserved() {
        let project = Project::bare();
        let agents_path = project.root.join("AGENTS.md");
        fs::write(&agents_path, "# operator prose\n").expect("stage operator AGENTS.md");

        let body = run::<workflow::init::handlers::Scaffold, _>(
            &project,
            workflow::init::handlers::ScaffoldInput {
                adapter: None,
                name: Some("demo-workspace".into()),
                description: None,
                workspace: true,
                platforms: None,
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
            !project.root.join(".specify/context.lock").exists(),
            "no fingerprint sidecar is written when generation is skipped"
        );
    }
}

mod journal {
    use super::*;

    #[tokio::test]
    async fn emit_appends_line() {
        let project = Project::initialised();
        let body = run::<workflow::journal::handlers::Emit, _>(
            &project,
            workflow::journal::handlers::EmitInput {
                event: "slice.build.started".into(),
                payload: Some(r#"{"slice-name":"billing"}"#.into()),
            },
        )
        .await
        .expect("emit succeeds");
        assert_eq!(body.event, "slice.build.started");
        let journal =
            fs::read_to_string(project.root.join(".specify/journal.jsonl")).expect("journal");
        assert!(
            journal.contains(r#""event":"slice.build.started""#),
            "the event landed:\n{journal}"
        );
    }

    #[tokio::test]
    async fn emit_unknown_event_refused() {
        let project = Project::initialised();
        let err = run::<workflow::journal::handlers::Emit, _>(
            &project,
            workflow::journal::handlers::EmitInput {
                event: "no.such.event".into(),
                payload: None,
            },
        )
        .await
        .expect_err("an id outside the closed taxonomy is refused");
        assert!(
            matches!(err.core(), error::Error::Validation { .. }),
            "an unknown id is the validation failure (exit 2 at the CLI), got {err:?}"
        );
    }

    #[tokio::test]
    async fn emit_bad_payload_refused() {
        let project = Project::initialised();
        let err = run::<workflow::journal::handlers::Emit, _>(
            &project,
            workflow::journal::handlers::EmitInput {
                event: "slice.build.started".into(),
                payload: Some("{}".into()),
            },
        )
        .await
        .expect_err("a payload missing the variant's fields is refused");
        assert!(
            matches!(err.core(), error::Error::Validation { .. }),
            "a bad payload is the validation failure (exit 2 at the CLI), got {err:?}"
        );
        assert!(
            !project.root.join(".specify/journal.jsonl").exists(),
            "a refused emit appends nothing"
        );
    }

    #[tokio::test]
    async fn show_reads_filtered() {
        let project = Project::initialised();
        run::<workflow::journal::handlers::Emit, _>(
            &project,
            workflow::journal::handlers::EmitInput {
                event: "slice.build.started".into(),
                payload: Some(r#"{"slice-name":"billing"}"#.into()),
            },
        )
        .await
        .expect("emit succeeds");
        let matched = run::<workflow::journal::handlers::Show, _>(
            &project,
            workflow::journal::handlers::ShowInput {
                filter: Some("slice.build".into()),
                limit: None,
            },
        )
        .await
        .expect("show succeeds");
        assert_eq!(matched.count, 1, "the emitted event matches its prefix");
        let unmatched = run::<workflow::journal::handlers::Show, _>(
            &project,
            workflow::journal::handlers::ShowInput {
                filter: Some("plan.".into()),
                limit: None,
            },
        )
        .await
        .expect("a filter with no matches still succeeds");
        assert_eq!(unmatched.count, 0, "no plan events were emitted");
        assert!(project.root.join(".specify/journal.jsonl").is_file(), "the journal persists");
    }
}

//! Init adapter argument coverage through the public scaffold operation.

use std::fs;

mod common;

use common::{Project, run, scoped_store, stage_dev_component};

fn input(adapter: &str) -> project::init::handlers::InitInput {
    project::init::handlers::InitInput {
        adapter: Some(adapter.to_string()),
        name: Some("demo-project".to_string()),
        description: None,
        workspace: false,
        platforms: None,
        upgrade: false,
    }
}

#[tokio::test]
async fn github_uri_refused() {
    let project = Project::bare();
    let err = run::<project::init::handlers::Init, _>(
        &project,
        input("https://github.com/augentic/specify/adapters/targets/demo"),
    )
    .await
    .expect_err("GitHub URI must be refused");

    assert_eq!(err.core().variant_str(), "adapter-github-uri-unsupported");
}

mod package {
    use super::*;

    #[tokio::test]
    async fn unhydratable_refused() {
        // The store misses the pin and the registry has no staged
        // response: init fails typed on the hydration leg.
        let project = Project::bare();
        let store = tempfile::tempdir().expect("store");
        let _guard = scoped_store(store.path());

        let err =
            run::<project::init::handlers::Init, _>(&project, input("specify:demo-target@1.2.0"))
                .await
                .expect_err("an unhydratable package must be refused");

        assert_eq!(err.core().variant_str(), "adapter-hydrate-failed");
    }

    #[tokio::test]
    async fn hydrated_from_registry() {
        // The store misses the pin; the staged registry response is
        // fetched, installed with its digest sidecar, verified, and
        // resolution proceeds against the fresh entry.
        let project = Project::bare();
        let store = tempfile::tempdir().expect("store");
        let _guard = scoped_store(store.path());
        let staged = project.root.join("hydrator/demo@1.2.0.wasm");
        fs::create_dir_all(staged.parent().expect("parent")).expect("mkdir hydrator");
        fs::write(&staged, b"\0asm-component").expect("stage registry response");

        let body = run::<project::init::handlers::Init, _>(&project, input("specify:demo@1.2.0"))
            .await
            .expect("hydrated package scaffolds");
        assert_eq!(body.hydrated, vec!["demo@1.2.0".to_string()]);

        let entry = diagnostics::cache::adapter_store_entry("demo", "1.2.0");
        assert_eq!(
            fs::read(&entry).expect("installed store entry"),
            b"\0asm-component",
            "the store entry is the fetched bytes"
        );
        assert!(
            entry.with_extension("meta").is_file(),
            "the digest sidecar lands beside the entry"
        );

        // A second init over a fresh project reuses the installed
        // entry without fetching.
        let project = Project::bare();
        let body = run::<project::init::handlers::Init, _>(&project, input("specify:demo@1.2.0"))
            .await
            .expect("installed package scaffolds without fetching");
        assert!(body.hydrated.is_empty(), "an installed pin fetches nothing");
    }

    #[tokio::test]
    async fn installed_resolves() {
        let project = Project::bare();
        let store = tempfile::tempdir().expect("store");
        let _guard = scoped_store(store.path());
        let entry = diagnostics::cache::adapter_store_entry("demo", "1.2.0");
        fs::write(&entry, b"\0asm-component").expect("stage installed component");

        run::<project::init::handlers::Init, _>(&project, input("specify:demo@1.2.0"))
            .await
            .expect("installed package scaffolds");

        let project_yaml =
            fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
        assert!(
            project_yaml.contains("adapter: specify:demo@1.2.0"),
            "package reference is canonical:\n{project_yaml}"
        );

        let body = run::<project::adapter::handlers::TargetResolve, _>(
            &project,
            project::adapter::handlers::ResolveInput {
                value: "specify:demo@1.2.0".to_string(),
                project_dir: None,
            },
        )
        .await
        .expect("recorded package reference resolves");
        assert_eq!(body.name, "demo");
        assert_eq!(body.version, "1.2.0");
        assert_eq!(body.location, "store");
    }
}

mod shorthand {
    use super::*;

    #[tokio::test]
    async fn canonicalised() {
        let project = Project::bare();
        stage_dev_component(&project.root, "demo");

        run::<project::init::handlers::Init, _>(&project, input("demo"))
            .await
            .expect("bare development shorthand scaffolds");
        let project_yaml =
            fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
        assert!(project_yaml.contains("adapter: demo"), "{project_yaml}");

        let project = Project::bare();
        let store = tempfile::tempdir().expect("store");
        let _guard = scoped_store(store.path());
        fs::write(diagnostics::cache::adapter_store_entry("demo", "1.2.0"), b"\0asm-component")
            .expect("stage installed component");

        run::<project::init::handlers::Init, _>(&project, input("demo@1.2.0"))
            .await
            .expect("versioned shorthand scaffolds");
        let project_yaml =
            fs::read_to_string(project.root.join(".specify/project.yaml")).expect("project.yaml");
        assert!(project_yaml.contains("adapter: specify:demo@1.2.0"), "{project_yaml}");
    }

    #[tokio::test]
    async fn invalid_not_registry_sugar() {
        for value in ["Demo-target", "demo-target@latest", "demo-target@1"] {
            let project = Project::bare();
            let err = run::<project::init::handlers::Init, _>(&project, input(value))
                .await
                .expect_err("invalid shorthand must not resolve as registry sugar");
            assert_eq!(err.core().variant_str(), "adapter-component-missing", "{value}");
        }
    }
}

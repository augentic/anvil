//! Init adapter argument coverage through the public scaffold operation.

use std::fs;

mod support;

use mock::invoke::run;
use support::{Provider, stage_cached_component};

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
    let project = Provider::bare();
    let err = run::<project::init::handlers::Init, _, _>(
        &project,
        input("https://github.com/augentic/emery/adapters/targets/demo"),
    )
    .await
    .expect_err("GitHub URI must be refused");

    assert_eq!(err.core().variant_str(), "adapter-github-uri-unsupported");
}

mod package {
    use super::*;

    #[tokio::test]
    async fn cold_pin_resolves_without_store() {
        // Package metadata dispatches by routed id before any store
        // read — under the shipped deployment the host resolver
        // installs a missing pin during that dispatch, so a cold
        // store neither blocks init nor leaves a guest-written entry.
        let project = Provider::bare();

        let body = run::<project::init::handlers::Init, _, _>(&project, input("emery:demo@1.2.0"))
            .await
            .expect("a cold package pin scaffolds");
        assert_eq!(body.adapter_name, "demo");
        assert!(
            !project.store_entry("demo", "1.2.0").exists(),
            "the guest writes no store entry — package install is host-owned"
        );

        let project_yaml =
            fs::read_to_string(project.root.join(".emery/project.yaml")).expect("project.yaml");
        assert!(
            project_yaml.contains("adapter: emery:demo@1.2.0"),
            "package reference is canonical:\n{project_yaml}"
        );
    }

    #[tokio::test]
    async fn pin_resolves_as_store_identity() {
        let project = Provider::bare();

        run::<project::init::handlers::Init, _, _>(&project, input("emery:demo@1.2.0"))
            .await
            .expect("package pin scaffolds");

        let body = run::<project::adapter::handlers::TargetResolve, _, _>(
            &project,
            project::adapter::handlers::ResolveInput {
                value: "emery:demo@1.2.0".to_string(),
                project_dir: None,
            },
        )
        .await
        .expect("recorded package reference resolves");
        assert_eq!(body.name, "demo");
        assert_eq!(body.version.as_deref(), Some("1.2.0"));
        assert_eq!(body.location, "store");
    }
}

mod shorthand {
    use super::*;

    #[tokio::test]
    async fn canonicalised() {
        let project = Provider::bare();
        stage_cached_component(&project, "demo");

        run::<project::init::handlers::Init, _, _>(&project, input("demo"))
            .await
            .expect("bare cached shorthand scaffolds");
        let project_yaml =
            fs::read_to_string(project.root.join(".emery/project.yaml")).expect("project.yaml");
        assert!(project_yaml.contains("adapter: demo"), "{project_yaml}");

        let project = Provider::bare();
        run::<project::init::handlers::Init, _, _>(&project, input("demo@1.2.0"))
            .await
            .expect("versioned shorthand scaffolds");
        let project_yaml =
            fs::read_to_string(project.root.join(".emery/project.yaml")).expect("project.yaml");
        assert!(project_yaml.contains("adapter: emery:demo@1.2.0"), "{project_yaml}");
    }

    #[tokio::test]
    async fn invalid_not_registry_sugar() {
        for value in ["Demo-target", "demo-target@latest", "demo-target@1"] {
            let project = Provider::bare();
            let err = run::<project::init::handlers::Init, _, _>(&project, input(value))
                .await
                .expect_err("invalid shorthand must not resolve as registry sugar");
            assert_eq!(err.core().variant_str(), "adapter-component-missing", "{value}");
        }
    }
}

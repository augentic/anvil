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
    async fn cold_pin_without_store() {
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

mod bare {
    use project::handler::Render as _;

    use super::*;

    fn rendered(body: &project::init::handlers::InitBody) -> String {
        let mut buffer = Vec::new();
        body.render(&mut buffer).expect("render init body");
        String::from_utf8(buffer).expect("rendered body is UTF-8")
    }

    #[tokio::test]
    async fn cache_miss_persists_bare() {
        // A bare name with no seeded cache entry resolves
        // dispatch-first — the deployment locates the component
        // local-first on the other side of the seam — and persists
        // exactly as typed: no version pin is invented.
        let project = Provider::bare();
        let body = run::<project::init::handlers::Init, _, _>(&project, input("demo"))
            .await
            .expect("bare cache-miss init resolves dispatch-first");

        assert_eq!(body.adapter_binding.as_deref(), Some("demo"));
        assert!(rendered(&body).contains("adapter: demo"), "{}", rendered(&body));
        let project_yaml =
            fs::read_to_string(project.root.join(".emery/project.yaml")).expect("project.yaml");
        assert!(project_yaml.contains("adapter: demo"), "{project_yaml}");
    }

    #[tokio::test]
    async fn cache_hit_persists_bare() {
        // The `adapter add` co-dev seed always wins: a cache hit
        // stays bare on disk and in the output.
        let project = Provider::bare();
        stage_cached_component(&project, "demo");
        let body = run::<project::init::handlers::Init, _, _>(&project, input("demo"))
            .await
            .expect("bare cache-hit init stays bare");

        assert_eq!(body.adapter_binding.as_deref(), Some("demo"));
        let project_yaml =
            fs::read_to_string(project.root.join(".emery/project.yaml")).expect("project.yaml");
        assert!(project_yaml.contains("adapter: demo"), "{project_yaml}");
    }

    #[tokio::test]
    async fn upgrade_bare_cleared_cache() {
        // `--upgrade` re-ensures the recorded binding: a bare record
        // whose cache entry was cleared resolves dispatch-first and
        // stays bare — no pin is written into the record.
        let project = Provider::bare();
        stage_cached_component(&project, "demo");
        run::<project::init::handlers::Init, _, _>(&project, input("demo"))
            .await
            .expect("bare cache-hit init");

        let entry = support::expected_cache_dir(&project).join("components/demo.wasm");
        fs::remove_file(&entry).expect("clear the seeded cache entry");

        let body = run::<project::init::handlers::Init, _, _>(
            &project,
            project::init::handlers::InitInput {
                upgrade: true,
                ..Default::default()
            },
        )
        .await
        .expect("upgrade re-ensures the bare record dispatch-first");

        assert_eq!(body.adapter_binding.as_deref(), Some("demo"));
        let project_yaml =
            fs::read_to_string(project.root.join(".emery/project.yaml")).expect("project.yaml");
        assert!(project_yaml.contains("adapter: demo"), "{project_yaml}");
    }

    #[tokio::test]
    async fn upgrade_bare_cache_hit() {
        // Co-dev upgrades stay bare: the record is never rewritten.
        let project = Provider::bare();
        stage_cached_component(&project, "demo");
        run::<project::init::handlers::Init, _, _>(&project, input("demo"))
            .await
            .expect("bare cache-hit init");

        let body = run::<project::init::handlers::Init, _, _>(
            &project,
            project::init::handlers::InitInput {
                upgrade: true,
                ..Default::default()
            },
        )
        .await
        .expect("upgrade over a live cache seed");

        assert_eq!(body.adapter_binding.as_deref(), Some("demo"));
        let project_yaml =
            fs::read_to_string(project.root.join(".emery/project.yaml")).expect("project.yaml");
        assert!(project_yaml.contains("adapter: demo"), "{project_yaml}");
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

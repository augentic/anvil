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

mod train {
    use project::adapter::FIRST_PARTY_ADAPTER_TRAIN;
    use project::handler::Render as _;

    use super::*;

    fn rendered(body: &project::init::handlers::InitBody) -> String {
        let mut buffer = Vec::new();
        body.render(&mut buffer).expect("render init body");
        String::from_utf8(buffer).expect("rendered body is UTF-8")
    }

    #[tokio::test]
    async fn bare_cache_miss_pins_and_persists() {
        // A bare first-party name with no seeded cache entry expands
        // to the embedded adapter-train pin at ensure time; the pin
        // is persisted (record-before-use) and self-announced in the
        // rendered output.
        let project = Provider::bare();
        let body = run::<project::init::handlers::Init, _, _>(&project, input("demo"))
            .await
            .expect("bare cache-miss init auto-pins");

        let pin = format!("emery:demo@{FIRST_PARTY_ADAPTER_TRAIN}");
        assert_eq!(body.adapter_binding.as_deref(), Some(pin.as_str()));
        assert!(rendered(&body).contains(&format!("adapter: {pin}")), "{}", rendered(&body));
        let project_yaml =
            fs::read_to_string(project.root.join(".emery/project.yaml")).expect("project.yaml");
        assert!(project_yaml.contains(&format!("adapter: {pin}")), "{project_yaml}");
    }

    #[tokio::test]
    async fn bare_cache_hit_persists_bare() {
        // The `adapter add` co-dev seed always wins: a cache hit
        // stays bare on disk and in the output.
        let project = Provider::bare();
        stage_cached_component(&project, "demo");
        let body = run::<project::init::handlers::Init, _, _>(&project, input("demo"))
            .await
            .expect("bare cache-hit init stays bare");

        assert_eq!(body.adapter_binding.as_deref(), Some("demo"));
        assert!(!body.adapter_binding_rewritten);
        let project_yaml =
            fs::read_to_string(project.root.join(".emery/project.yaml")).expect("project.yaml");
        assert!(project_yaml.contains("adapter: demo"), "{project_yaml}");
    }

    #[tokio::test]
    async fn upgrade_rewrites_bare_record_on_cleared_cache() {
        // `--upgrade` re-ensures the recorded binding: a bare record
        // whose cache entry was cleared expands to the train pin, and
        // the record is rewritten (and the rewrite rendered) to keep
        // record-before-use.
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
        .expect("upgrade expands the drifted record");

        let pin = format!("emery:demo@{FIRST_PARTY_ADAPTER_TRAIN}");
        assert!(body.adapter_binding_rewritten, "the drifted record is rewritten");
        assert_eq!(body.adapter_binding.as_deref(), Some(pin.as_str()));
        assert!(
            rendered(&body).contains(&format!("adapter binding rewritten to {pin}")),
            "{}",
            rendered(&body)
        );
        let project_yaml =
            fs::read_to_string(project.root.join(".emery/project.yaml")).expect("project.yaml");
        assert!(project_yaml.contains(&format!("adapter: {pin}")), "{project_yaml}");
    }

    #[tokio::test]
    async fn upgrade_keeps_bare_record_on_cache_hit() {
        // Co-dev upgrades stay bare: a cache-hit bare record expands
        // to itself, so nothing is rewritten.
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

        assert!(!body.adapter_binding_rewritten);
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

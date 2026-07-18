//! Adapter resolution and metadata gates through public surfaces.

mod support;

use mock::invoke::run;
use project::adapter::{AdapterSelector, Resolver};
use project::handler::{Anchor as _, ExecutionPaths};
use support::{Provider, stage_dev_component};

mod resolve {
    use super::*;

    #[tokio::test]
    async fn metadata_gated() {
        for (name, expected) in
            [("demo-target", "adapter-cli-too-old"), ("bad-floor", "adapter-floor-malformed")]
        {
            let project = Provider::bare();
            stage_dev_component(&project.root, name);
            let err = run::<project::adapter::handlers::TargetResolve, _, _>(
                &project,
                project::adapter::handlers::ResolveInput {
                    value: name.to_string(),
                    project_dir: None,
                },
            )
            .await
            .expect_err("metadata gate must reject the adapter");
            assert_eq!(err.core().variant_str(), expected);
        }
    }

    #[tokio::test]
    async fn bare_development_identity() {
        let project = Provider::bare();
        stage_dev_component(&project.root, "demo");

        let body = run::<project::adapter::handlers::TargetResolve, _, _>(
            &project,
            project::adapter::handlers::ResolveInput {
                value: "demo".to_string(),
                project_dir: None,
            },
        )
        .await
        .expect("bare development adapter resolves");

        assert_eq!(body.name, "demo");
        assert_eq!(body.version, "0.0.0");
        assert_eq!(body.axis, "targets");
    }

    #[tokio::test]
    async fn bare_from_project_cache() {
        // The other bare-name probe: a component mirrored into the
        // project component cache (init's local-file path) resolves
        // without any in-repo release build.
        let project = Provider::bare();
        let components = support::expected_cache_dir(&project).join("components");
        std::fs::create_dir_all(&components).expect("mkdir component cache");
        std::fs::write(components.join("demo.wasm"), "{}").expect("stage cached component");

        let body = run::<project::adapter::handlers::TargetResolve, _, _>(
            &project,
            project::adapter::handlers::ResolveInput {
                value: "demo".to_string(),
                project_dir: None,
            },
        )
        .await
        .expect("cached component resolves");
        assert_eq!(body.name, "demo");
    }

    #[test]
    fn sibling_checkout_never_probed() {
        // Resolution is project-contained: an artifact in a sibling
        // `specify-adapters` checkout (the retired development probe)
        // must not resolve a bare name.
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let outer = tmp.path().canonicalize().expect("canonical tempdir");
        let project_dir = outer.join("project");
        std::fs::create_dir_all(&project_dir).expect("mkdir project");
        let sibling = outer.join("specify-adapters/target/wasm32-wasip2/release");
        std::fs::create_dir_all(&sibling).expect("mkdir sibling layout");
        std::fs::write(sibling.join("demo.wasm"), "{}").expect("stage sibling component");

        let paths = ExecutionPaths::isolated(&project_dir, outer.join("project-cache"));
        let err = support::resolver()
            .resolve_target(&AdapterSelector::parse("demo").expect("bare selector"), &paths)
            .expect_err("sibling artifact must not resolve");
        assert!(err.to_string().contains("adapter-not-found"), "{err}");
    }
}

#[test]
fn platforms_metadata_preserved() {
    let project = Provider::bare();
    stage_dev_component(&project.root, "vectis");

    let resolved = support::resolver()
        .resolve_target(&AdapterSelector::parse("vectis").expect("bare selector"), project.paths())
        .expect("vectis resolves");
    let platforms = resolved.manifest.platforms.expect("platform capability");

    assert!(platforms.required);
    assert_eq!(
        platforms.allowed,
        [project::Platform::Core, project::Platform::Ios, project::Platform::Android]
    );
}

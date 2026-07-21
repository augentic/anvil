//! Adapter resolution and metadata gates through public surfaces.

mod support;

use mock::invoke::run;
use project::adapter::{AdapterSelector, Resolver};
use project::handler::{Anchor as _, CachePlacement, ExecutionPaths, Locations};
use support::{Provider, stage_cached_component};

mod resolve {
    use super::*;

    #[tokio::test]
    async fn metadata_gated() {
        for (name, expected) in
            [("demo-target", "adapter-cli-too-old"), ("bad-floor", "adapter-floor-malformed")]
        {
            let project = Provider::bare();
            stage_cached_component(&project, name);
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
    async fn bare_cache_identity() {
        // A bare name resolves the seeded project component cache and
        // carries no package identity: the envelope omits `version`
        // and labels the origin `cache`.
        let project = Provider::bare();
        stage_cached_component(&project, "demo");

        let body = run::<project::adapter::handlers::TargetResolve, _, _>(
            &project,
            project::adapter::handlers::ResolveInput {
                value: "demo".to_string(),
                project_dir: None,
            },
        )
        .await
        .expect("bare cached adapter resolves");

        assert_eq!(body.name, "demo");
        assert_eq!(body.version, None, "an unpinned cache resolve has no version");
        assert_eq!(body.location, "cache");
        assert_eq!(body.axis, "targets");
    }

    #[test]
    fn dev_build_never_probed() {
        // The retired Cargo probe: an in-repo release artifact at
        // `target/wasm32-wasip2/release/<name>.wasm` must not satisfy
        // production resolution — only the seeded cache or a pin does.
        let project = Provider::bare();
        let dev_dir = project.root.join("target/wasm32-wasip2/release");
        std::fs::create_dir_all(&dev_dir).expect("mkdir dev release dir");
        std::fs::write(dev_dir.join("demo.wasm"), "{}").expect("stage dev artifact");

        let err = support::resolver()
            .resolve_target(
                &AdapterSelector::parse("demo").expect("bare selector"),
                project.paths(),
            )
            .expect_err("a release-build artifact must not resolve");
        assert!(err.to_string().contains("adapter-not-found"), "{err}");
        assert!(err.to_string().contains("adapter add"), "the miss suggests seeding: {err}");
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

        let locations = Locations::explicit(
            outer.join("store"),
            CachePlacement::Parent(outer.join("project-cache")),
        );
        let paths = ExecutionPaths::new(&project_dir, locations);
        let err = support::resolver()
            .resolve_target(&AdapterSelector::parse("demo").expect("bare selector"), &paths)
            .expect_err("sibling artifact must not resolve");
        assert!(err.to_string().contains("adapter-not-found"), "{err}");
    }
}

#[test]
fn platforms_metadata_preserved() {
    let project = Provider::bare();
    stage_cached_component(&project, "vectis");

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

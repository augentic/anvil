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
    async fn wrong_axis_fails_at_dispatch() {
        // The store carries no axis segment, so a component bound on
        // the wrong axis fails at the dispatch seam: no deployed
        // guest exports the requested `<axis>:<name>` id, the
        // metadata dispatch errors, and the resolve aborts.
        let project = Provider::bare();
        let err = run::<project::adapter::handlers::SourceResolve, _, _>(
            &project,
            project::adapter::handlers::ResolveInput {
                value: "emery:demo-target@1.2.0".to_string(),
                project_dir: None,
            },
        )
        .await
        .expect_err("wrong-axis dispatch must fail");
        assert_eq!(err.core().variant_str(), "adapter-metadata-failed");
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

    #[tokio::test]
    async fn bare_cache_miss_dispatches() {
        // A bare name with no seeded cache entry resolves
        // dispatch-first: the deployment locates the component
        // local-first behind the seam, so the envelope carries no
        // version (the guest never learns which one settled) and the
        // origin names the routed id, not a file.
        let project = Provider::bare();

        let body = run::<project::adapter::handlers::TargetResolve, _, _>(
            &project,
            project::adapter::handlers::ResolveInput {
                value: "demo".to_string(),
                project_dir: None,
            },
        )
        .await
        .expect("bare cache-miss adapter resolves dispatch-first");

        assert_eq!(body.name, "demo");
        assert_eq!(body.version, None, "a bare dispatch-first resolve has no version");
        assert_eq!(body.location, "store");
        assert_eq!(body.resolved_path, "target:demo");
    }

    #[test]
    fn dev_build_never_probed() {
        // The retired Cargo probe: an in-repo release artifact at
        // `target/wasm32-wasip2/release/<name>.wasm` must not satisfy
        // production resolution — a bare cache miss dispatches the
        // routed id instead of probing build trees.
        let project = Provider::bare();
        let dev_dir = project.root.join("target/wasm32-wasip2/release");
        std::fs::create_dir_all(&dev_dir).expect("mkdir dev release dir");
        std::fs::write(dev_dir.join("demo.wasm"), "{}").expect("stage dev artifact");

        let resolved = support::resolver()
            .resolve_target(
                &AdapterSelector::parse("demo").expect("bare selector"),
                project.paths(),
            )
            .expect("bare cache miss resolves dispatch-first");
        assert_eq!(
            resolved.origin.reference, "target:demo",
            "the dev artifact never backs the resolve"
        );
    }

    #[test]
    fn sibling_checkout_never_probed() {
        // Resolution is project-contained: an artifact in a sibling
        // `emery-adapters` checkout (the retired development probe)
        // must not back a bare-name resolve.
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let outer = tmp.path().canonicalize().expect("canonical tempdir");
        let project_dir = outer.join("project");
        std::fs::create_dir_all(&project_dir).expect("mkdir project");
        let sibling = outer.join("emery-adapters/target/wasm32-wasip2/release");
        std::fs::create_dir_all(&sibling).expect("mkdir sibling layout");
        std::fs::write(sibling.join("demo.wasm"), "{}").expect("stage sibling component");

        let locations = Locations::explicit(
            outer.join("store"),
            CachePlacement::Parent(outer.join("project-cache")),
        );
        let paths = ExecutionPaths::new(&project_dir, locations);
        let resolved = support::resolver()
            .resolve_target(&AdapterSelector::parse("demo").expect("bare selector"), &paths)
            .expect("bare cache miss resolves dispatch-first");
        assert_eq!(
            resolved.origin.reference, "target:demo",
            "the sibling artifact never backs the resolve"
        );
    }
}

mod update {
    use super::*;

    async fn update(
        project: &Provider, name: &str,
    ) -> Result<project::adapter::handlers::ResolveBody, project::handler::Error> {
        run::<project::adapter::handlers::AdapterUpdate, _, _>(
            project,
            project::adapter::handlers::UpdateInput {
                name: name.to_string(),
            },
        )
        .await
    }

    #[tokio::test]
    async fn bare_target_resolves() {
        // The handler is axis-neutral over the unique name space:
        // the target axis answers first, dispatch-first with no
        // version (the deployment settles the version behind the
        // seam and reports it on stderr).
        let project = Provider::bare();
        let body = update(&project, "demo").await.expect("bare update resolves");
        assert_eq!(body.axis, "targets");
        assert_eq!(body.name, "demo");
        assert_eq!(body.version, None);
    }

    #[tokio::test]
    async fn falls_back_to_the_source_axis() {
        // A source adapter's name fails the target-axis dispatch (no
        // deployed guest exports `target:<name>`) and resolves on the
        // source axis.
        let project = Provider::bare();
        let body = update(&project, "demo-source").await.expect("source-axis update resolves");
        assert_eq!(body.axis, "sources");
        assert_eq!(body.name, "demo-source");
    }

    #[tokio::test]
    async fn non_bare_values_refused() {
        // Pins are immutable and local components update through
        // `adapter add`.
        let project = Provider::bare();
        for value in ["emery:demo@1.0.0", "demo@1.0.0", "./demo.wasm"] {
            let err = update(&project, value).await.expect_err("non-bare update must refuse");
            assert_eq!(err.core().variant_str(), "adapter-update-not-bare", "{value}");
        }
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

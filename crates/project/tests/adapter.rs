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
    async fn wrong_axis_fails_dispatch() {
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
    async fn bare_cache_miss() {
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
    fn sibling_checkout_never() {
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

mod upgrade {
    use project::adapter::handlers::{AdapterUpgrade, UpgradeBody, UpgradeInput};

    use super::*;

    async fn upgrade(
        project: &Provider, input: UpgradeInput,
    ) -> Result<UpgradeBody, project::handler::Error> {
        run::<AdapterUpgrade, _, _>(project, input).await
    }

    fn named(name: &str) -> UpgradeInput {
        UpgradeInput {
            name: Some(name.to_string()),
            all: false,
            project_dir: None,
        }
    }

    const fn all() -> UpgradeInput {
        UpgradeInput {
            name: None,
            all: true,
            project_dir: None,
        }
    }

    /// Stage `.emery/project.yaml` with the given adapter binding.
    fn stage_project(provider: &Provider, adapter: &str) {
        let emery = provider.root.join(".emery");
        std::fs::create_dir_all(&emery).expect("mkdir .emery");
        std::fs::write(
            emery.join("project.yaml"),
            format!("name: demo-project\nadapter: {adapter}\n"),
        )
        .expect("write project.yaml");
    }

    #[tokio::test]
    async fn bare_target_resolves() {
        // The handler is axis-neutral over the unique name space:
        // the target axis answers first, dispatch-first with no
        // version (the deployment settles the version behind the
        // seam and reports it on stderr).
        let project = Provider::bare();
        let body = upgrade(&project, named("demo")).await.expect("bare upgrade resolves");
        assert_eq!(body.adapters.len(), 1);
        assert_eq!(body.adapters[0].axis, "targets");
        assert_eq!(body.adapters[0].name, "demo");
        assert_eq!(body.adapters[0].version, None);
    }

    #[tokio::test]
    async fn falls_back_source_axis() {
        // A source adapter's name fails the target-axis dispatch (no
        // deployed guest exports `target:<name>`) and resolves on the
        // source axis.
        let project = Provider::bare();
        let body =
            upgrade(&project, named("demo-source")).await.expect("source-axis upgrade resolves");
        assert_eq!(body.adapters[0].axis, "sources");
        assert_eq!(body.adapters[0].name, "demo-source");
    }

    #[tokio::test]
    async fn non_bare_values_refused() {
        // Pins are immutable and local components refresh through
        // `adapter add`.
        let project = Provider::bare();
        for value in ["emery:demo@1.0.0", "demo@1.0.0", "./demo.wasm"] {
            let err =
                upgrade(&project, named(value)).await.expect_err("non-bare upgrade must refuse");
            assert_eq!(err.core().variant_str(), "adapter-upgrade-not-bare", "{value}");
        }
    }

    #[tokio::test]
    async fn all_collects_bare() {
        // `--all` walks the project's recorded bare bindings: the
        // `project.yaml` target. Plan source rows are exact pins and
        // never contribute.
        let project = Provider::bare();
        stage_project(&project, "demo");
        let plan_path = project.root.join(".emery/change/plan.yaml");
        std::fs::create_dir_all(plan_path.parent().expect("parent")).expect("change home");
        std::fs::write(
            &plan_path,
            "name: demo-plan\n\
             sources:\n\
             \x20 brief:\n\
             \x20   adapter: emery:demo-source@0.0.0\n\
             \x20   value: operator brief\n\
             \x20 pinned:\n\
             \x20   adapter: emery:demo@9.9.9\n\
             \x20   locator: src/\n\
             slices: []\n",
        )
        .expect("write plan.yaml");

        let body = upgrade(&project, all()).await.expect("--all resolves the bare bindings");
        let names: Vec<&str> = body.adapters.iter().map(|a| a.name.as_str()).collect();
        assert_eq!(names, ["demo"], "only the project.yaml bare target");
        assert_eq!(body.adapters[0].axis, "targets");
    }

    #[tokio::test]
    async fn all_empty_nothing_bare() {
        // A pinned project binding and no plan leave nothing to
        // upgrade: exit 0 with an empty set, not an error.
        let project = Provider::bare();
        stage_project(&project, "emery:demo@1.0.0");
        let body = upgrade(&project, all()).await.expect("--all over pins succeeds empty");
        assert!(body.adapters.is_empty());
    }

    #[tokio::test]
    async fn all_requires_a_project() {
        // The collection walks recorded bindings, so `--all` outside
        // an initialized project is `not-initialized`.
        let project = Provider::bare();
        let err = upgrade(&project, all()).await.expect_err("--all needs project.yaml");
        assert_eq!(err.core().variant_str(), "not-initialized");
    }
}

#[test]
fn platforms_metadata() {
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

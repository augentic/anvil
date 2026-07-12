//! Adapter resolution and metadata gates through public surfaces.

mod common;

use common::{Project, run, stage_dev_component};
use workflow::adapter::Resolver;

mod resolve {
    use super::*;

    #[tokio::test]
    async fn metadata_gated() {
        for (name, expected) in
            [("demo-target", "adapter-cli-too-old"), ("bad-floor", "adapter-floor-malformed")]
        {
            let project = Project::bare();
            stage_dev_component(&project.root, name);
            let err = run::<workflow::adapter::handlers::TargetResolve, _>(
                &project,
                workflow::adapter::handlers::ResolveInput {
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
        let project = Project::bare();
        stage_dev_component(&project.root, "demo");

        let body = run::<workflow::adapter::handlers::TargetResolve, _>(
            &project,
            workflow::adapter::handlers::ResolveInput {
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
}

#[test]
fn platforms_metadata_preserved() {
    let project = Project::bare();
    stage_dev_component(&project.root, "vectis");

    let resolved = common::resolver()
        .resolve_target(&workflow::adapter::AdapterRef::bare("vectis"), &project.root)
        .expect("vectis resolves");
    let platforms = resolved.manifest.platforms.expect("platform capability");

    assert!(platforms.required);
    assert_eq!(
        platforms.allowed,
        [workflow::Platform::Core, workflow::Platform::Ios, workflow::Platform::Android]
    );
}

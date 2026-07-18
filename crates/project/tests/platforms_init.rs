//! Target platform gates through the public scaffold operation.

mod support;

use mock::invoke::run;
use support::{Provider, stage_dev_component};

#[tokio::test]
async fn vectis_platform_requirements() {
    let cases = [
        (None, "project-platforms-required"),
        (Some("ios,android"), "project-platforms-must-include-core"),
        (Some("core,web"), "project-platforms-not-allowed"),
    ];

    for (platforms, expected) in cases {
        let project = Provider::bare();
        stage_dev_component(&project.root, "vectis");
        let err = run::<project::init::handlers::Init, _, _>(
            &project,
            project::init::handlers::InitInput {
                adapter: Some("vectis".to_string()),
                name: Some("demo".to_string()),
                description: None,
                workspace: false,
                platforms: platforms.map(ToString::to_string),
                upgrade: false,
            },
        )
        .await
        .expect_err("invalid platform set must fail");

        assert_eq!(err.core().variant_str(), expected);
    }
}

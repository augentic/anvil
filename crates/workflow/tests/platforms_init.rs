//! Target platform gates through the public scaffold operation.

mod common;

use common::{Project, run, stage_dev_component};

#[tokio::test]
async fn vectis_platform_requirements() {
    let cases = [
        (None, "project-platforms-required"),
        (Some("ios,android"), "project-platforms-must-include-core"),
        (Some("core,web"), "project-platforms-not-allowed"),
    ];

    for (platforms, expected) in cases {
        let project = Project::bare();
        stage_dev_component(&project.root, "vectis");
        let err = run::<workflow::init::handlers::Scaffold, _>(
            &project,
            workflow::init::handlers::ScaffoldInput {
                adapter: Some("vectis".to_string()),
                name: Some("demo".to_string()),
                description: None,
                workspace: false,
                platforms: platforms.map(ToString::to_string),
            },
        )
        .await
        .expect_err("invalid platform set must fail");

        assert_eq!(err.core().variant_str(), expected);
    }
}

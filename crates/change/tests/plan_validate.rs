//! Plan structural checks through the public validate operation.

use std::fs;

use testkit::{ScriptedProvider, report_rule_ids, run};

#[tokio::test]
async fn structural_findings() {
    let cases = [
        (
            "duplicate-name",
            "slices:\n  - name: alpha\n    status: pending\n  - name: alpha\n    status: pending\n",
        ),
        (
            "unknown-depends-on",
            "slices:\n  - name: alpha\n    depends-on: [missing]\n    status: pending\n",
        ),
        (
            "unknown-source",
            "slices:\n  - name: alpha\n    sources: [missing]\n    status: pending\n",
        ),
        (
            "duplicate-source-key",
            "sources:\n  docs:\n    adapter: demo-source\n    path: /tmp/docs\nslices:\n  - name: alpha\n    sources: [docs, docs]\n    status: pending\n",
        ),
        (
            "cycle-in-depends-on",
            "slices:\n  - name: alpha\n    depends-on: [beta]\n    status: pending\n  - name: beta\n    depends-on: [alpha]\n    status: pending\n",
        ),
        (
            "cycle-in-depends-on",
            "slices:\n  - name: alpha\n    depends-on: [alpha]\n    status: pending\n",
        ),
    ];

    for (expected, body) in cases {
        let project = ScriptedProvider::initialised();
        fs::write(
            project.root.join("plan.yaml"),
            format!("name: validation\nlifecycle: pending\n{body}"),
        )
        .expect("stage plan");

        let err = run::<change::plan::handlers::Validate, _, _>(
            &project,
            change::plan::handlers::ValidateInput {},
        )
        .await
        .expect_err("invalid plan must fail");
        let ids = report_rule_ids(&err);
        assert!(ids.iter().any(|id| id == expected), "expected {expected} in {ids:?}");
    }
}

//! Plan structural checks through the public validate operation.

use std::fs;

mod support;

use mock::invoke::run;
use mock::session::Session;
use support::report_rule_ids;

#[tokio::test]
async fn structural_findings() {
    let cases = [
        ("duplicate-name", "slices:\n  - name: alpha\n  - name: alpha\n"),
        ("unknown-depends-on", "slices:\n  - name: alpha\n    depends-on: [missing]\n"),
        ("unknown-source", "slices:\n  - name: alpha\n    sources: [missing]\n"),
        (
            "duplicate-source-key",
            "sources:\n  docs:\n    adapter: demo-source\n    path: /tmp/docs\nslices:\n  - name: alpha\n    sources: [docs, docs]\n",
        ),
        (
            "cycle-in-depends-on",
            "slices:\n  - name: alpha\n    depends-on: [beta]\n  - name: beta\n    depends-on: [alpha]\n",
        ),
        ("cycle-in-depends-on", "slices:\n  - name: alpha\n    depends-on: [alpha]\n"),
    ];

    for (expected, body) in cases {
        let project = Session::scripted("demo", Vec::new());
        let path = project.root().join(".emery/change/plan.yaml");
        fs::create_dir_all(path.parent().expect("parent")).expect("change home");
        fs::write(&path, format!("name: validation\n{body}")).expect("stage plan");

        let err = run::<change::plan::handlers::Validate, _, _>(
            project.provider(),
            change::plan::handlers::ValidateInput {},
        )
        .await
        .expect_err("invalid plan must fail");
        let ids = report_rule_ids(&err);
        assert!(ids.iter().any(|id| id == expected), "expected {expected} in {ids:?}");
    }
}

//! Metadata timestamp serde through public lifecycle operations.

use std::fs;

mod common;

use common::{Project, run};

mod list {
    use super::*;

    /// `slice list` enumerates every slice with a `metadata.yaml`,
    /// sorted by name, and skips directories without one.
    #[tokio::test]
    async fn sorted_with_status() {
        let project = Project::initialised();
        for (name, status) in [("beta", "refined"), ("alpha", "refining")] {
            let slice = project.root.join(".specify/slices").join(name);
            fs::create_dir_all(&slice).expect("create slice");
            fs::write(
                slice.join("metadata.yaml"),
                format!("target: demo@1.0.0\nstatus: {status}\ntouched-specs: []\n"),
            )
            .expect("stage metadata");
        }
        fs::create_dir_all(project.root.join(".specify/slices/not-a-slice"))
            .expect("stage stray dir");

        let body = run::<workflow::slice::handlers::List, _>(
            &project,
            workflow::slice::handlers::ListInput::default(),
        )
        .await
        .expect("list succeeds");

        let listed: Vec<(&str, &str)> =
            body.slices.iter().map(|e| (e.name.as_str(), e.status.as_str())).collect();
        assert_eq!(listed, vec![("alpha", "refining"), ("beta", "refined")]);
    }
}

mod timestamps {
    use super::*;

    /// Timestamps written by earlier phases survive a lifecycle write
    /// byte-canonically (`slice drop` is the surviving public
    /// metadata-mutating verb).
    #[tokio::test]
    async fn round_trip_rfc3339() {
        let project = Project::initialised();
        let slice = project.root.join(".specify/slices/demo");
        fs::create_dir_all(&slice).expect("create slice");
        fs::write(
            slice.join("metadata.yaml"),
            "target: demo\nstatus: refining\ncreated-at: 2026-06-02T01:02:03+00:00\n\
         touched-specs: []\noutcome:\n  phase: guidance\n  outcome: success\n  \
         at: 2026-06-02T01:02:03+00:00\n  summary: ready\n",
        )
        .expect("stage metadata");

        run::<workflow::slice::handlers::Drop, _>(
            &project,
            workflow::slice::handlers::DropInput {
                name: "demo".to_string(),
                reason: None,
            },
        )
        .await
        .expect("metadata parses and the drop saves");

        let archived = project.root.join(".specify/archive");
        let dir = fs::read_dir(&archived)
            .expect("archive dir")
            .next()
            .expect("archived slice")
            .expect("entry")
            .path();
        let yaml = fs::read_to_string(dir.join("metadata.yaml")).expect("metadata");
        assert!(yaml.contains("created-at: 2026-06-02T01:02:03Z"), "{yaml}");
        assert!(yaml.contains("at: 2026-06-02T01:02:03Z"), "{yaml}");
    }

    #[tokio::test]
    async fn malformed_rejected() {
        let project = Project::initialised();
        let slice = project.root.join(".specify/slices/demo");
        fs::create_dir_all(&slice).expect("create slice");
        fs::write(
            slice.join("metadata.yaml"),
            "target: demo\nstatus: refining\ncreated-at: not-a-timestamp\ntouched-specs: []\n",
        )
        .expect("stage metadata");

        let err = run::<workflow::slice::handlers::Drop, _>(
            &project,
            workflow::slice::handlers::DropInput {
                name: "demo".to_string(),
                reason: None,
            },
        )
        .await
        .expect_err("malformed timestamp must fail");

        assert!(matches!(err.core(), error::Error::YamlDe(_)), "{err:?}");
    }
}

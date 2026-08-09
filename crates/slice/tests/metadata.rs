//! Metadata timestamp serde through public lifecycle operations.

use std::fs;

use mock::invoke::run;
use mock::session::Session;

mod list {
    use super::*;

    /// `slice list` enumerates every slice with a `metadata.yaml`,
    /// sorted by name, and skips directories without one.
    #[tokio::test]
    async fn sorted_with_status() {
        let project = Session::scripted("mock", Vec::new());
        for (name, refined) in [("beta", true), ("alpha", false)] {
            let slice = project.root().join(".emery/slices").join(name);
            fs::create_dir_all(&slice).expect("create slice");
            fs::write(slice.join("metadata.yaml"), "target: demo@1.0.0\ntouched-specs: []\n")
                .expect("stage metadata");
            if refined {
                fs::write(slice.join("model.yaml"), "requirements: []\n").expect("model");
            }
        }
        fs::create_dir_all(project.root().join(".emery/slices/not-a-slice"))
            .expect("stage stray dir");

        let body = run::<slice::handlers::List, _, _>(
            project.provider(),
            slice::handlers::ListInput::default(),
        )
        .await
        .expect("list succeeds");

        let listed: Vec<(&str, slice::LifecycleStatus)> =
            body.slices.iter().map(|e| (e.name.as_str(), e.status)).collect();
        assert_eq!(
            listed,
            vec![
                ("alpha", slice::LifecycleStatus::Refining),
                ("beta", slice::LifecycleStatus::Refined)
            ]
        );
    }
}

mod timestamps {
    use super::*;

    fn now() -> jiff::Timestamp {
        jiff::Timestamp::from_second(1_780_000_000).expect("timestamp")
    }

    /// Timestamps written by earlier phases survive a lifecycle write
    /// byte-canonically (`discard` — the `plan drop` kernel — is the
    /// surviving public metadata-mutating operation).
    #[test]
    fn round_trip_rfc3339() {
        let project = Session::scripted("mock", Vec::new());
        let slice_dir = project.root().join(".emery/slices/demo");
        fs::create_dir_all(&slice_dir).expect("create slice");
        fs::write(
            slice_dir.join("metadata.yaml"),
            "target: demo\ncreated-at: 2026-06-02T01:02:03+00:00\n\
         touched-specs: []\noutcome:\n  phase: guidance\n  outcome: success\n  \
         at: 2026-06-02T01:02:03+00:00\n  summary: ready\n",
        )
        .expect("stage metadata");

        let archive = project.root().join(".emery/archive");
        slice::discard(&slice_dir, &archive, None, now())
            .expect("metadata parses and the drop saves");

        let dir = fs::read_dir(&archive)
            .expect("archive dir")
            .next()
            .expect("archived slice")
            .expect("entry")
            .path();
        let yaml = fs::read_to_string(dir.join("metadata.yaml")).expect("metadata");
        assert!(yaml.contains("created-at: 2026-06-02T01:02:03Z"), "{yaml}");
        assert!(yaml.contains("at: 2026-06-02T01:02:03Z"), "{yaml}");
    }

    #[test]
    fn malformed_rejected() {
        let project = Session::scripted("mock", Vec::new());
        let slice_dir = project.root().join(".emery/slices/demo");
        fs::create_dir_all(&slice_dir).expect("create slice");
        fs::write(
            slice_dir.join("metadata.yaml"),
            "target: demo\ncreated-at: not-a-timestamp\ntouched-specs: []\n",
        )
        .expect("stage metadata");

        let archive = project.root().join(".emery/archive");
        let err = slice::discard(&slice_dir, &archive, None, now())
            .expect_err("malformed timestamp must fail");

        assert!(matches!(err, error::Error::YamlDe(_)), "{err:?}");
    }
}

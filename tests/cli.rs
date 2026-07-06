//! Integration tests for the top-level CLI surface (dispatch + error
//! mapping). Shared helpers live in [`common`].

mod base {
    //! Cross-cutting binary-contract tests for the `specify` CLI.
    //!
    //! These are the subcommand-agnostic invariants every release must
    //! preserve: the top-level `--help` shape, exit-code contracts that
    //! aren't tied to a single subcommand, and JSON-envelope skeletons
    //! that any verb may surface. Per-subcommand integration coverage
    //! lives in dedicated `tests/<subcommand>.rs` files.

    use std::fs;

    use tempfile::tempdir;

    use crate::common::{help_verbs, omnia_component, specify_cmd};

    #[test]
    fn help_exits_zero_and_prints_usage() {
        // No exact clap wording: assert exit 0 and that the help text lists
        // the core top-level verbs.
        let verbs = help_verbs(&[]);
        for verb in ["init", "plan", "slice", "source", "target", "workspace"] {
            assert!(verbs.iter().any(|v| v == verb), "--help must list `{verb}`, got: {verbs:?}");
        }
    }

    #[test]
    fn version_too_old_exits_three_json() {
        // Generic exit-code 3 + JSON error-envelope contract: pin the
        // `specify-version-too-old` shape via a real project. Routed
        // through `slice validate` because that path runs the version
        // gate after a successful init; the gate itself is subcommand-
        // agnostic and the assertions below only touch the envelope.
        let tmp = tempdir().unwrap();
        // Fresh init to produce a real project.
        specify_cmd()
            .current_dir(tmp.path())
            .args(["init"])
            .arg(omnia_component())
            .args(["--name", "demo"])
            .assert()
            .success();

        // Pin a version far in the future.
        let config_path = tmp.path().join(".specify/project.yaml");
        let original = fs::read_to_string(&config_path).unwrap();
        let edited =
            original.replace(&format!("specify: {}", env!("CARGO_PKG_VERSION")), "specify: 99.0.0");
        fs::write(&config_path, edited).unwrap();

        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "slice", "validate", "."])
            .assert()
            .failure();
        let code = assert.get_output().status.code().expect("process exited with a code");
        assert_eq!(code, 3, "expected exit code 3 (version too old)");

        // Failure envelopes are written to stderr.
        let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("utf8");
        let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr is JSON");
        assert_eq!(value["error"], "specify-version-too-old");
        assert_eq!(value["exit-code"], 3);
    }

    #[test]
    fn older_pin_loads_fine() {
        // A pin older than the binary is legal: pre-1.0 majors are re-init,
        // not migration, so config load succeeds and the command proceeds to
        // ordinary slice validation (exit 2 on the bogus slice name).
        let tmp = tempdir().unwrap();
        specify_cmd()
            .current_dir(tmp.path())
            .args(["init"])
            .arg(omnia_component())
            .args(["--name", "demo"])
            .assert()
            .success();

        let config_path = tmp.path().join(".specify/project.yaml");
        let original = fs::read_to_string(&config_path).unwrap();
        let edited =
            original.replace(&format!("specify: {}", env!("CARGO_PKG_VERSION")), "specify: 0.0.1");
        assert_ne!(original, edited, "fixture must repin specify");
        fs::write(&config_path, edited).unwrap();

        let assert = specify_cmd()
            .current_dir(tmp.path())
            .args(["--format", "json", "slice", "validate", "."])
            .assert()
            .failure();
        let code = assert.get_output().status.code().expect("process exited with a code");
        assert_eq!(code, 2, "older pin loads; the failure is ordinary validation");
        let stderr = String::from_utf8(assert.get_output().stderr.clone()).expect("utf8");
        let value: serde_json::Value = serde_json::from_str(&stderr).expect("stderr is JSON");
        assert_eq!(value["error"], "slice-validation-failed");
    }
}

mod errors {
    //! Locks down `Exit::from(&Error)` — the single source of truth for the
    //! CLI's process exit codes (AGENTS.md §"Exit codes"). One representative
    //! `Error` variant per code keeps the wire contract from drifting.

    use specify::runtime::Exit;
    use specify_error::Error;

    /// Each row pairs a representative `Error` with the exit code its
    /// `Exit::from` mapping must yield.
    fn error_exit_cases() -> Vec<(&'static str, Error, u8)> {
        vec![
            // 1 — generic failure: every variant without a dedicated arm.
            ("io", Error::Io(std::io::Error::other("boom")), 1),
            (
                "diag",
                Error::Diag {
                    code: "some-diag",
                    detail: "detail".to_string(),
                },
                1,
            ),
            ("not-initialized", Error::NotInitialized, 1),
            (
                "filesystem",
                Error::Filesystem {
                    op: "readdir",
                    path: std::path::PathBuf::from("/nope"),
                    source: std::io::Error::other("io"),
                },
                1,
            ),
            // 2 — validation failed.
            ("validation", Error::validation_failed("bad-thing", "rule", "detail"), 2),
            // 2 — argument errors share the validation exit code.
            (
                "argument",
                Error::Argument {
                    flag: "--adapter",
                    detail: "unknown".to_string(),
                },
                2,
            ),
            // 3 — CLI older than the project floor.
            (
                "cli-too-old",
                Error::CliTooOld {
                    required: "1.0.0".to_string(),
                    found: "0.9.0".to_string(),
                },
                3,
            ),
            // 3 — CLI older than an adapter's `specify` floor (RFC-47 D3).
            (
                "adapter-cli-too-old",
                Error::AdapterCliTooOld {
                    adapter: "omnia (omnia@1.0.0.wasm)".to_string(),
                    required: "2.0.0".to_string(),
                    found: "1.0.0".to_string(),
                },
                3,
            ),
        ]
    }

    #[test]
    fn error_variants_map_to_exit_codes() {
        for (label, err, expected) in error_exit_cases() {
            assert_eq!(
                Exit::from(&err).code(),
                expected,
                "Error::{label} must map to exit code {expected}"
            );
        }
    }

    #[test]
    fn success_is_zero() {
        // `Exit::from(&Error)` only covers the failure path; success comes
        // from the non-error branch and must stay 0.
        assert_eq!(Exit::Success.code(), 0);
    }

    #[test]
    fn every_documented_code_is_covered() {
        let mut codes: Vec<u8> = error_exit_cases().into_iter().map(|(_, _, code)| code).collect();
        codes.push(Exit::Success.code());
        codes.sort_unstable();
        codes.dedup();
        assert_eq!(codes, vec![0, 1, 2, 3], "exit-code table must cover 0–3");
    }
}

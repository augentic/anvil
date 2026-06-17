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
                adapter: "omnia (adapter.yaml)".to_string(),
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

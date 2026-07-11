//! Integration coverage for the context-fingerprint kernel
//! (`workflow::agents::fingerprint`): the canonical hashing matrix, the
//! collector's dedup / sort / soft-skip behavior, and the
//! outside-project diagnostic — all through the public API.

use std::fs;

use error::Error;
use workflow::agents::fingerprint::{
    InputCollector, InputFingerprint, aggregate, body_sha256, for_context,
};

fn input(path: &str, sha256: &str) -> InputFingerprint {
    InputFingerprint {
        path: path.to_string(),
        sha256: sha256.to_string(),
    }
}

// The pure hashing functions are a `(inputs/version/body -> digest)`
// matrix with no CLI fixture pinning the canonical encoding, so the hash
// cases live here as one matrix.
#[test]
fn hashing_matrix() {
    // `aggregate` sorts inputs by path before hashing, yielding a stable digest.
    let inputs = vec![
        input("registry.yaml", "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"),
        input(
            ".specify/project.yaml",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ),
    ];
    assert_eq!(
        aggregate("0.2.0", inputs),
        "sha256:96f096c433da7e43d6ab7ce7aa305882f3eb2933fa160d00640af8a0df17e73f"
    );

    // It is order-independent: the same multiset hashes identically.
    let alpha = input(
        ".specify/project.yaml",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    );
    let beta =
        input("registry.yaml", "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
    let gamma =
        input("Cargo.toml", "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc");
    assert_eq!(
        aggregate("0.2.0", vec![alpha.clone(), beta.clone(), gamma.clone()]),
        aggregate("0.2.0", vec![gamma, alpha, beta])
    );

    // The CLI version is the first aggregate line, so a bump alone changes
    // the digest even when every input digest is identical.
    let pinned = vec![input(
        ".specify/project.yaml",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    )];
    assert_ne!(aggregate("0.2.0", pinned.clone()), aggregate("0.3.0", pinned));

    // `body_sha256` changes with the fenced body bytes.
    assert_ne!(
        body_sha256(b"\n## Runtime\n- detected: Rust.\n\n"),
        body_sha256(b"\n## Runtime\n- detected: Node.js.\n\n")
    );

    // `for_context` wires cli_version / inputs / aggregate / body digest.
    let inputs = vec![input(
        "registry.yaml",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    )];
    let body = b"\n## Runtime\n- detected: Rust.\n\n";
    let fp = for_context("0.2.0", inputs.clone(), body);
    assert_eq!(fp.cli_version, "0.2.0");
    assert_eq!(fp.inputs, inputs);
    assert_eq!(fp.fingerprint, aggregate("0.2.0", inputs));
    assert_eq!(fp.body_sha256, body_sha256(body));
}

// The collector keys inputs by repo-relative path and dedups, then
// `finalize` hashes content in sorted path order; `add_file_if_present` is
// the soft variant that silently skips a missing path and a directory. A
// regression that dropped the dedup or the sort would shuffle the canonical
// aggregate and break lock stability across runs.
#[test]
fn collector_dedups_and_filters() {
    let project = tempfile::tempdir().expect("tempdir");
    let root = project.path();
    fs::write(root.join("z.txt"), b"zed").expect("write z");
    fs::create_dir_all(root.join("sub")).expect("sub");
    fs::write(root.join("sub/a.txt"), b"aaa").expect("write a");

    let mut collector = InputCollector::new(root);
    collector.add_file(&root.join("z.txt")).expect("add z");
    collector.add_file(&root.join("sub/a.txt")).expect("add a");
    // Adding the same file again must not produce a second entry.
    collector.add_file(&root.join("z.txt")).expect("re-add z");
    let inputs = collector.finalize().expect("finalize");
    assert_eq!(
        inputs.iter().map(|i| i.path.as_str()).collect::<Vec<_>>(),
        vec!["sub/a.txt", "z.txt"]
    );
    assert_eq!(inputs[1].sha256, schema::digest::sha256_hex(b"zed"));

    let project = tempfile::tempdir().expect("tempdir");
    let root = project.path();
    fs::create_dir_all(root.join("a-dir")).expect("dir");
    fs::write(root.join("real.txt"), b"x").expect("file");
    let mut collector = InputCollector::new(root);
    collector.add_file_if_present(&root.join("missing.txt")).expect("missing skipped");
    collector.add_file_if_present(&root.join("a-dir")).expect("dir skipped");
    collector.add_file_if_present(&root.join("real.txt")).expect("real recorded");
    let inputs = collector.finalize().expect("finalize");
    assert_eq!(inputs.iter().map(|i| i.path.as_str()).collect::<Vec<_>>(), vec!["real.txt"]);
}

// An input path outside the project root is a programmer error and
// must surface as the typed `context-fingerprint-input-outside-project`
// diagnostic rather than producing a bogus relative path.
#[test]
fn input_outside_project_errors() {
    let project = tempfile::tempdir().expect("project");
    let other = tempfile::tempdir().expect("other");
    fs::write(other.path().join("stray.txt"), b"x").expect("write stray");

    let mut collector = InputCollector::new(project.path());
    let err = collector.add_file(&other.path().join("stray.txt")).expect_err("outside project");
    assert!(
        matches!(
            err,
            Error::Diag {
                code: "context-fingerprint-input-outside-project",
                ..
            }
        ),
        "{err}"
    );
}

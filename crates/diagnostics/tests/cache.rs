//! Digest-keyed cache coverage.

use emery_diagnostics::cache::content_digest;

#[test]
fn content_digests_deterministic() {
    let digest = content_digest(b"wasm-bytes-1");

    assert!(digest.starts_with("sha256:"), "{digest}");
    assert_eq!(digest, content_digest(b"wasm-bytes-1"));
    assert_ne!(digest, content_digest(b"wasm-bytes-2"));
}

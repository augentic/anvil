//! The verify-on-read sidecar format: digest math and sidecar-text
//! parsing over the public bytes-in surface.

use emery_diagnostics::cache::{
    OciProvenance, content_digest, recorded_digest, recorded_provenance,
};

#[test]
fn content_digests_deterministic() {
    let digest = content_digest(b"wasm-bytes-1");

    assert!(digest.starts_with("sha256:"), "{digest}");
    assert_eq!(digest, content_digest(b"wasm-bytes-1"));
    assert_ne!(digest, content_digest(b"wasm-bytes-2"));
}

#[test]
fn sidecar_parses_digest_and_provenance() {
    let sidecar = "tree_digest: sha256:abc123\noci:\n  repository: ghcr.io/acme/adapters/demo\n  \
                   manifest_digest: sha256:manifest\n  layer_digest: sha256:layer\n";

    assert_eq!(recorded_digest(sidecar).as_deref(), Some("sha256:abc123"));
    assert_eq!(
        recorded_provenance(sidecar),
        Some(OciProvenance {
            repository: "ghcr.io/acme/adapters/demo".to_string(),
            manifest_digest: "sha256:manifest".to_string(),
            layer_digest: "sha256:layer".to_string(),
        })
    );
}

#[test]
fn sidecar_without_provenance_parses() {
    let sidecar = "tree_digest: sha256:abc123\n";

    assert_eq!(recorded_digest(sidecar).as_deref(), Some("sha256:abc123"));
    assert_eq!(recorded_provenance(sidecar), None);
}

#[test]
fn malformed_sidecar_is_none() {
    assert_eq!(recorded_digest("not: [a, sidecar"), None);
    assert_eq!(recorded_digest(""), None);
}

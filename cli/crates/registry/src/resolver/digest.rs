//! Digest computation and verification for resolver-acquired bytes.

use std::fs;
use std::path::Path;

use specify_schema::digest::sha256_hex;

use crate::error::ExtensionError;
use crate::package::AcquiredBytes;

pub(super) fn cached_matches(
    module: &Path, expected: Option<&str>,
) -> Result<bool, ExtensionError> {
    let Some(expected) = expected else {
        return Ok(true);
    };
    let bytes = fs::read(module).map_err(|err| {
        ExtensionError::cache_io("read cached module for sha256 verification", module, err)
    })?;
    Ok(sha256_hex(&bytes) == expected)
}

pub(super) fn validate(
    source: &str, acquired: &AcquiredBytes, expected_sha256: Option<&str>,
) -> Result<(), ExtensionError> {
    if acquired.len()? == 0 {
        return Err(ExtensionError::Diag {
            code: "tool-resolver",
            detail: format!("tool source `{source}` produced empty bytes"),
        });
    }

    if let Some(expected) = expected_sha256
        && acquired.sha256 != expected
    {
        return Err(ExtensionError::Diag {
            code: "tool-resolver",
            detail: format!(
                "tool source `{source}` sha256 mismatch: expected {expected}, got {}",
                acquired.sha256
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;
    use std::io::Write as _;
    use std::path::Path;

    use super::super::resolve;
    use super::*;
    use crate::error::ExtensionError;
    use crate::manifest::ExtensionSource;
    use crate::test_support::{
        cache_env, cached_bytes, fixed_now, project_scope, scratch_dir, tool, write_source,
    };

    fn corrupt_cached_module(path: &Path, bytes: &[u8]) {
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(path)
            .expect("open cached module for corruption");
        file.write_all(bytes).expect("overwrite cached module");
    }

    #[test]
    fn digest_mismatch_preserves_cache() {
        let cache_dir = scratch_dir("resolver-digest-cache");
        let project_dir = scratch_dir("resolver-digest-project");
        let source_dir = scratch_dir("resolver-digest-source");
        let old_source = write_source(&source_dir, "old.wasm", b"old-good");
        let new_source = write_source(&source_dir, "new.wasm", b"new-good");
        let old_sha = sha256_hex(b"old-good");
        let wrong_sha =
            "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff".to_string();
        let scope = project_scope();
        let old_tool = tool(ExtensionSource::LocalPath(old_source), Some(old_sha));
        let wrong_tool = tool(ExtensionSource::LocalPath(new_source.clone()), Some(wrong_sha));

        let _env = cache_env(&cache_dir);

        resolve(&scope, &old_tool, fixed_now(), &project_dir).expect("initial digest install");
        let err = resolve(&scope, &wrong_tool, fixed_now(), &project_dir)
            .expect_err("wrong digest must fail");
        assert!(
            matches!(&err, ExtensionError::Diag { code: "tool-resolver", detail }
                if detail.contains("sha256 mismatch")),
            "{err}"
        );
        assert_eq!(cached_bytes(&scope, &old_tool), b"old-good");

        let correct_tool =
            tool(ExtensionSource::LocalPath(new_source), Some(sha256_hex(b"new-good")));
        resolve(&scope, &correct_tool, fixed_now(), &project_dir)
            .expect("correct digest updates cache");
        assert_eq!(cached_bytes(&scope, &correct_tool), b"new-good");
    }

    fn acquired(bytes: &[u8]) -> AcquiredBytes {
        let temp = tempfile::NamedTempFile::new().expect("create acquired tempfile");
        fs::write(temp.path(), bytes).expect("write acquired bytes");
        AcquiredBytes {
            temp,
            sha256: sha256_hex(bytes),
            package_metadata: None,
        }
    }

    // `validate` is the digest gate every acquired source flows through.
    // The empty-bytes guard, the pinned-mismatch guard, and the two
    // pass-through arms (pinned match / unpinned) each gate a different
    // failure mode; assert all four directly so a refactor of the early
    // returns cannot collapse one into another.
    #[test]
    fn validate_gates_empty_and_mismatch() {
        let good = acquired(b"good-bytes");
        let empty = acquired(b"");
        let correct = sha256_hex(b"good-bytes");
        let wrong = "f".repeat(64);

        let empty_err = validate("src", &empty, None).expect_err("empty bytes are rejected");
        assert!(
            matches!(&empty_err, ExtensionError::Diag { code: "tool-resolver", detail }
                if detail.contains("produced empty bytes")),
            "{empty_err}"
        );

        let mismatch =
            validate("src", &good, Some(&wrong)).expect_err("pinned mismatch is rejected");
        assert!(
            matches!(&mismatch, ExtensionError::Diag { code: "tool-resolver", detail }
                if detail.contains("sha256 mismatch")),
            "{mismatch}"
        );

        validate("src", &good, Some(&correct)).expect("matching pin passes");
        validate("src", &good, None).expect("unpinned non-empty passes");
    }

    // `cached_matches` short-circuits to `Ok(true)` for an unpinned tool
    // without touching the filesystem, but surfaces a typed I/O error
    // when a pinned tool's cached module is missing.
    #[test]
    fn cached_matches_pin_handling() {
        let absent = Path::new("/definitely/missing/module.wasm");
        assert!(cached_matches(absent, None).expect("unpinned hit ignores missing bytes"));

        let err =
            cached_matches(absent, Some(&"a".repeat(64))).expect_err("pinned miss must read bytes");
        assert!(matches!(err, ExtensionError::Diag { code: "tool-io", .. }), "{err}");
    }

    #[test]
    fn cached_bytes_rehashed_on_pinned_hit() {
        let cache_dir = scratch_dir("resolver-hit-digest-cache");
        let project_dir = scratch_dir("resolver-hit-digest-project");
        let source_dir = scratch_dir("resolver-hit-digest-source");
        let source = write_source(&source_dir, "module.wasm", b"trusted");
        let scope = project_scope();
        let pinned = tool(ExtensionSource::LocalPath(source), Some(sha256_hex(b"trusted")));

        let _env = cache_env(&cache_dir);

        let resolved = resolve(&scope, &pinned, fixed_now(), &project_dir).expect("install pinned");
        corrupt_cached_module(&resolved.bytes_path, b"corrupt");
        let repaired = resolve(&scope, &pinned, fixed_now(), &project_dir)
            .expect("digest mismatch re-fetches");
        assert_eq!(repaired.bytes_path, resolved.bytes_path);
        assert_eq!(fs::read(repaired.bytes_path).expect("repaired bytes"), b"trusted");
    }
}

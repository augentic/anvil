//! Local-path and `file://` URI source resolution.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::ExtensionError;

pub(super) fn read_file_uri(uri: &str) -> Result<Vec<u8>, ExtensionError> {
    let Some(rest) = uri.strip_prefix("file://") else {
        return Err(ExtensionError::Diag {
            code: "tool-resolver",
            detail: format!(
                "tool source `{uri}` is invalid: file URI sources must start with file://"
            ),
        });
    };
    if rest.is_empty() {
        return Err(ExtensionError::Diag {
            code: "tool-resolver",
            detail: format!(
                "tool source `{uri}` is invalid: file URI source path must not be empty"
            ),
        });
    }
    read_local_path(&PathBuf::from(rest), uri)
}

pub(super) fn read_local_path(path: &Path, source: &str) -> Result<Vec<u8>, ExtensionError> {
    if !path.is_absolute() {
        return Err(ExtensionError::Diag {
            code: "tool-resolver",
            detail: format!(
                "tool source `{source}` is invalid: local source path must be absolute"
            ),
        });
    }
    let metadata = fs::metadata(path)
        .map_err(|err| ExtensionError::cache_io("inspect local source", path, err))?;
    if !metadata.is_file() {
        return Err(ExtensionError::Diag {
            code: "tool-resolver",
            detail: format!(
                "tool source `{source}` is invalid: local source path must resolve to a regular file"
            ),
        });
    }
    fs::read(path).map_err(|err| ExtensionError::cache_io("read local source", path, err))
}

#[cfg(test)]
mod tests {
    use super::super::resolve;
    use crate::error::ExtensionError;
    use crate::manifest::ExtensionSource;
    use crate::test_support::{
        cache_env, fixed_now, named_tool, project_scope, scratch_dir, tool, write_source,
    };

    #[test]
    fn file_uri_reuses_local_path_resolution() {
        let cache_dir = scratch_dir("resolver-file-cache");
        let project_dir = scratch_dir("resolver-file-project");
        let source_dir = scratch_dir("resolver-file-source");
        let source = write_source(&source_dir, "module.wasm", b"file-uri");
        let scope = project_scope();
        let local = named_tool("local", ExtensionSource::LocalPath(source.clone()), None);
        let file_uri = named_tool(
            "file-uri",
            ExtensionSource::FileUri(format!("file://{}", source.display())),
            None,
        );

        let _env = cache_env(&cache_dir);

        let local = resolve(&scope, &local, fixed_now(), &project_dir).expect("local resolves");
        let uri = resolve(&scope, &file_uri, fixed_now(), &project_dir).expect("file URI resolves");
        assert_eq!(std::fs::read(local.bytes_path).expect("local bytes"), b"file-uri");
        assert_eq!(std::fs::read(uri.bytes_path).expect("uri bytes"), b"file-uri");
    }

    #[test]
    fn local_rejects_non_file() {
        let cache_dir = scratch_dir("resolver-invalid-local-cache");
        let project_dir = scratch_dir("resolver-invalid-local-project");
        let source_dir = scratch_dir("resolver-invalid-local-source");
        let empty = write_source(&source_dir, "empty.wasm", b"");
        let scope = project_scope();

        let _env = cache_env(&cache_dir);

        let dir_err = resolve(
            &scope,
            &tool(ExtensionSource::LocalPath(source_dir), None),
            fixed_now(),
            &project_dir,
        )
        .expect_err("directory source must fail");
        assert!(
            matches!(&dir_err, ExtensionError::Diag { code: "tool-resolver", detail }
                if detail.contains("must resolve to a regular file")),
            "{dir_err}"
        );

        let empty_err = resolve(
            &scope,
            &tool(ExtensionSource::LocalPath(empty), None),
            fixed_now(),
            &project_dir,
        )
        .expect_err("empty file");
        assert!(
            matches!(&empty_err, ExtensionError::Diag { code: "tool-resolver", detail }
                if detail.contains("produced empty bytes")),
            "{empty_err}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn local_chases_symlinks() {
        let cache_dir = scratch_dir("resolver-symlink-cache");
        let project_dir = scratch_dir("resolver-symlink-project");
        let source_dir = scratch_dir("resolver-symlink-source");
        let target = write_source(&source_dir, "target.wasm", b"symlink-target");
        let link = source_dir.join("link.wasm");
        std::os::unix::fs::symlink(&target, &link).expect("create symlink");
        let scope = project_scope();
        let symlink_tool = tool(ExtensionSource::LocalPath(link), None);

        let _env = cache_env(&cache_dir);

        let resolved =
            resolve(&scope, &symlink_tool, fixed_now(), &project_dir).expect("symlink resolves");
        assert_eq!(std::fs::read(resolved.bytes_path).expect("cached bytes"), b"symlink-target");
    }
}

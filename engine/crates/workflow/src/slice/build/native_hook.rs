//! Manifest-declared native build hooks.
//!
//! Target adapters may declare `host_prereq` and `finalize_verify` scripts
//! in `adapter.yaml`. The host CLI executes them with `SPECIFY_PROJECT_DIR`
//! and `SPECIFY_SLICE_DIR` set.

use std::path::{Component, Path, PathBuf};
use std::process::Command;

use specify_error::{Error, Result};

use crate::adapter::NativeBuildHookDeclaration;

/// Execute a manifest-declared native build hook script.
///
/// # Errors
///
/// Returns [`Error::Validation`] with `abort_code` when the script is
/// missing, escapes the adapter root, fails to spawn, or exits non-zero.
pub fn run_native_build_hook(
    adapter_root: &Path, hook: &NativeBuildHookDeclaration, project_dir: &Path, slice_dir: &Path,
    abort_code: &'static str, abort_expectation: &'static str,
) -> Result<()> {
    let script_path = resolve_hook_script(adapter_root, &hook.script)?;
    if !script_path.is_file() {
        return Err(Error::validation_failed(
            abort_code,
            abort_expectation,
            format!("hook script `{}` is not a file", script_path.display()),
        ));
    }

    let output = Command::new("sh")
        .arg(&script_path)
        .current_dir(adapter_root)
        .env("SPECIFY_PROJECT_DIR", project_dir)
        .env("SPECIFY_SLICE_DIR", slice_dir)
        .output()
        .map_err(|err| {
            Error::validation_failed(
                abort_code,
                abort_expectation,
                format!("failed to spawn hook script `{}`: {err}", script_path.display()),
            )
        })?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if stderr.trim().is_empty() {
        stdout.trim().to_string()
    } else {
        stderr.trim().to_string()
    };
    Err(Error::validation_failed(
        abort_code,
        abort_expectation,
        format!(
            "hook script `{}` exited with {}: {detail}",
            script_path.display(),
            output.status.code().unwrap_or(-1)
        ),
    ))
}

fn resolve_hook_script(adapter_root: &Path, script_rel: &str) -> Result<PathBuf> {
    let rel = Path::new(script_rel);
    if rel.is_absolute() || rel.components().any(|c| matches!(c, Component::ParentDir)) {
        return Err(Error::validation_failed(
            "adapter-manifest-invalid",
            "native build hook script paths must be relative and must not contain `..`",
            format!("invalid hook script path `{script_rel}`"),
        ));
    }
    Ok(adapter_root.join(rel))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use specify_error::Error;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn runs_successful_hook_script() {
        let adapter = tempdir().expect("tempdir");
        let scripts = adapter.path().join("scripts");
        fs::create_dir_all(&scripts).expect("mkdir scripts");
        let script = scripts.join("ok.sh");
        fs::write(&script, "#!/bin/sh\nexit 0\n").expect("write script");

        let project = tempdir().expect("project");
        let slice = project.path().join(".specify/slices/demo");
        fs::create_dir_all(&slice).expect("mkdir slice");

        run_native_build_hook(
            adapter.path(),
            &NativeBuildHookDeclaration {
                script: "scripts/ok.sh".into(),
            },
            project.path(),
            &slice,
            "target-build-host-prereq-missing",
            "host prereq hook passes",
        )
        .expect("hook ok");
    }

    #[test]
    fn rejects_parent_dir_escape() {
        let adapter = tempdir().expect("tempdir");
        let err = run_native_build_hook(
            adapter.path(),
            &NativeBuildHookDeclaration {
                script: "../escape.sh".into(),
            },
            adapter.path(),
            adapter.path(),
            "target-build-host-prereq-missing",
            "host prereq hook passes",
        )
        .expect_err("must reject escape");
        let Error::Validation { code, .. } = err else {
            panic!("expected validation error");
        };
        assert_eq!(code, "adapter-manifest-invalid");
    }
}

//! Shared per-step pipeline primitives.
//!
//! Chunks 7/8 captured the `make` / `gradle` step-by-step shape inside
//! `init::ios` and `init::android`. Chunk 9 lifts them here so both the
//! init-time post-scaffold smoke build and the verify-time per-assembly
//! compile pipeline share a single `BuildStep` shape and a single
//! `run_step` implementation. Anything that runs a command and wants to
//! land its pass/fail status inside `assemblies.<name>.steps` should go
//! through [`run_step`].

use std::process::Command;

use crate::error::VectisError;

/// One step of a per-assembly build pipeline.
///
/// Mirrors the JSON object the RFC prescribes for `assemblies.*.steps[*]`
/// so the dispatcher can splice the struct straight into the serialized
/// output.
///
/// `name` is intentionally `&'static str` -- every pipeline step label
/// is compile-time known (`"cargo check"`, `"make typegen"`, ...), so
/// keeping it borrowed keeps the struct cheap to clone and move around.
#[derive(Debug, Clone, serde::Serialize)]
pub struct BuildStep {
    pub name: &'static str,
    pub passed: bool,
    /// Combined stdout+stderr captured when the step fails. `None` on
    /// success keeps the happy-path JSON compact -- the field is
    /// `#[serde(skip_serializing_if = "Option::is_none")]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Run a single pipeline step and turn its exit status into a
/// [`BuildStep`].
///
/// The caller builds the `Command` (setting `current_dir`, `args`, and
/// any env vars it needs) and passes it in. On success the returned
/// `BuildStep` carries `passed: true` and no error string. On a non-zero
/// exit both stdout and stderr are captured into `error` so the agent
/// invoking vectis can surface the failure without a separate re-run.
///
/// `VectisError::Verify` is only produced when the process itself cannot
/// be spawned (missing binary, permission error) -- a non-zero exit from
/// the child is treated as a *step* failure, not a handler failure, so
/// the dispatcher can continue to the next assembly.
pub fn run_step(name: &'static str, cmd: &mut Command) -> Result<BuildStep, VectisError> {
    let output = cmd.output().map_err(|e| VectisError::Verify {
        message: format!("failed to invoke `{name}`: {e}"),
    })?;
    if output.status.success() {
        Ok(BuildStep {
            name,
            passed: true,
            error: None,
        })
    } else {
        let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
        if !combined.is_empty() && !combined.ends_with('\n') {
            combined.push('\n');
        }
        combined.push_str(&String::from_utf8_lossy(&output.stderr));
        Ok(BuildStep {
            name,
            passed: false,
            error: Some(combined.trim().to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_step_captures_stdout_and_stderr_on_failure() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "echo out; echo err 1>&2; exit 2"]);
        let step = run_step("sh fail", &mut cmd).unwrap();
        assert!(!step.passed);
        let err = step.error.expect("error must be present");
        assert!(err.contains("out"), "stdout missing from combined: {err}");
        assert!(err.contains("err"), "stderr missing from combined: {err}");
    }

    #[test]
    fn run_step_marks_success_with_no_error() {
        let mut cmd = Command::new("sh");
        cmd.args(["-c", "exit 0"]);
        let step = run_step("sh ok", &mut cmd).unwrap();
        assert!(step.passed);
        assert!(step.error.is_none());
    }

    #[test]
    fn run_step_surfaces_spawn_failures_as_verify_error() {
        let mut cmd = Command::new("/this/binary/does/not/exist");
        let err = run_step("missing", &mut cmd).expect_err("spawn must fail");
        match err {
            VectisError::Verify { message } => {
                assert!(message.contains("failed to invoke `missing`"), "{message}");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
}

use std::path::Path;
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkipDecision {
    pub skip: bool,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecifyResult {
    pub stdout: String,
    pub stderr: String,
    pub code: i32,
}

static CACHED_BIN: OnceLock<Option<String>> = OnceLock::new();

/// Resolve `SPECIFY_BIN` first, then `specify` on `PATH`.
pub fn resolve_specify_bin() -> Option<String> {
    CACHED_BIN
        .get_or_init(|| {
            if let Ok(override_path) = std::env::var("SPECIFY_BIN") {
                let path = Path::new(&override_path);
                if path.is_file() {
                    return Some(override_path);
                }
                return None;
            }

            Command::new("sh")
                .args(["-c", "command -v specify"])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .output()
                .ok()
                .filter(|output| output.status.success())
                .and_then(|output| {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if path.is_empty() {
                        None
                    } else {
                        Some(path)
                    }
                })
        })
        .clone()
}

/// Cached skip decision for subprocess replay tests.
pub fn skip_unless_specify_bin() -> SkipDecision {
    match resolve_specify_bin() {
        Some(_) => SkipDecision {
            skip: false,
            reason: None,
        },
        None => SkipDecision {
            skip: true,
            reason: Some(
                "specify binary not resolvable; set SPECIFY_BIN or install `specify` on PATH"
                    .into(),
            ),
        },
    }
}

/// Run `body` only when a `specify` binary is available; otherwise print a skip note.
pub fn with_specify_bin(body: impl FnOnce()) {
    let decision = skip_unless_specify_bin();
    if decision.skip {
        eprintln!("  skipped: {}", decision.reason.unwrap_or_default());
        return;
    }
    body();
}

/// Execute the resolved `specify` binary with `args`.
#[allow(dead_code)]
pub fn run_specify(args: &[&str], cwd: Option<&Path>) -> Result<SpecifyResult, String> {
    let bin = resolve_specify_bin().ok_or_else(|| {
        "specify binary not resolvable; set SPECIFY_BIN or install `specify` on PATH".to_string()
    })?;

    let mut command = Command::new(&bin);
    command.args(args);
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }

    let output = command
        .output()
        .map_err(|err| format!("run {bin}: {err}"))?;
    Ok(output_to_result(output))
}

fn output_to_result(output: Output) -> SpecifyResult {
    SpecifyResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        code: output.status.code().unwrap_or(-1),
    }
}

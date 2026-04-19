//! `vectis add-shell` -- add iOS or Android to an existing project.
//!
//! Chunk 10 lands the real handler. The implementation in one breath:
//!
//! 1. Resolve the project directory, validate `shared/src/app.rs` exists
//!    (the add-shell entry point cannot create a core; `vectis init` is
//!    the only path there).
//! 2. Refuse up front if the requested shell already exists so we never
//!    partially overwrite a user-edited `iOS/` or `Android/` tree.
//! 3. Parse `shared/src/app.rs` with [`parser::parse_app_rs`] to recover
//!    the app name + capability set -- `add-shell` deliberately does
//!    NOT ask the user to re-specify these (the RFC's common case is
//!    teams that start core-only and add shells months later, when
//!    nobody remembers the original `--caps`).
//! 4. Resolve version pins via the same `Versions::resolve` path `init`
//!    uses; a bad `--version-file` must fail before any byte is written.
//! 5. Dispatch to `init::ios::scaffold` or `init::android::scaffold` with
//!    `run_build=true`. The scaffold-time pipeline bootstraps the
//!    Android Gradle wrapper (chunk 8) and runs the per-shell make /
//!    xcodebuild / gradlew pipeline (chunks 7/8 + chunk 9's shared
//!    `verify::pipeline::BuildStep`), emitting the same `BuildStep`
//!    shape that `vectis verify` would produce for the just-added
//!    assembly. This satisfies the RFC's "run verify for the just-added
//!    assembly" requirement without duplicating the build -- the
//!    alternative ("scaffold(false) then verify::run(..)") requires
//!    separately bootstrapping the Android wrapper before verify's
//!    `./gradlew :app:assembleDebug` can find a `gradlew` binary, and
//!    doubles every build step when the wrapper *is* present.

pub mod parser;

use std::path::{Path, PathBuf};

use crate::{
    AddShellArgs, CommandOutcome,
    error::VectisError,
    init,
    prerequisites::{self, AssemblyKind},
    verify::pipeline::BuildStep,
    versions::Versions,
};

pub fn run(args: &AddShellArgs) -> Result<CommandOutcome, VectisError> {
    let shell = match args.platform.as_str() {
        "ios" => AssemblyKind::Ios,
        "android" => AssemblyKind::Android,
        other => {
            return Err(VectisError::InvalidProject {
                message: format!(
                    "unknown shell platform: {other:?} (expected one of: ios, android)"
                ),
            });
        }
    };

    let project_dir = resolve_project_dir(args.dir.as_deref())?;

    // Core must exist. The RFC is explicit: `add-shell` is for adding
    // to an existing project -- if there is no `shared/src/app.rs` we
    // point the user at `vectis init` rather than silently scaffolding
    // a half-project with just a shell.
    let app_rs_path = project_dir.join("shared").join("src").join("app.rs");
    if !app_rs_path.is_file() {
        return Err(VectisError::InvalidProject {
            message: format!(
                "no core assembly found at {} (run `vectis init` to scaffold a new project)",
                app_rs_path.display()
            ),
        });
    }

    // Refuse before scaffolding if the target shell already exists.
    // Mirrors the atomic-refusal guarantee each scaffold enforces on
    // its own -- we re-check up front so prerequisite probing doesn't
    // run when there's no work to do.
    let shell_root = project_dir.join(shell_dir_name(shell));
    if shell_root.exists() {
        return Err(VectisError::InvalidProject {
            message: format!(
                "shell already present at {} (delete it first or run `vectis verify`)",
                shell_root.display()
            ),
        });
    }

    // Prereqs: core + requested shell only (no point checking the
    // *other* shell's tooling here; verify will run its own scoped
    // check later if the project also has that other shell).
    prerequisites::check(&[AssemblyKind::Core, shell])?;

    // Parse `app.rs` so we can reuse the user's existing app name and
    // capability set -- the RFC's "don't ask the user to re-specify"
    // contract.
    let source = std::fs::read_to_string(&app_rs_path)?;
    let parsed = parser::parse_app_rs(&source)?;

    // Resolve version pins. This also validates a bad `--version-file`
    // early, before any byte is written.
    let versions = Versions::resolve(&project_dir, args.version_file.as_deref())?;

    let android_package = args
        .android_package
        .clone()
        .unwrap_or_else(|| init::core::default_android_package(&parsed.app_name));

    let params = init::build_params(&parsed.app_name, &android_package, &versions);

    // Scaffold the requested shell with `run_build=true`: the scaffold
    // writes files, bootstraps Android's Gradle wrapper if needed, and
    // then runs the shell's full build pipeline, capturing every step
    // into `BuildStep`s using the same struct `vectis verify` uses.
    let (files, build_steps, written_shell): (Vec<String>, Vec<BuildStep>, &'static str) =
        match shell {
            AssemblyKind::Ios => {
                let s = init::ios::scaffold(
                    &project_dir,
                    &parsed.app_name,
                    &parsed.capabilities,
                    &params,
                    true,
                )?;
                (s.files, s.build_steps, "ios")
            }
            AssemblyKind::Android => {
                let s = init::android::scaffold(
                    &project_dir,
                    &android_package,
                    &parsed.capabilities,
                    &params,
                    &versions,
                    true,
                )?;
                (s.files, s.build_steps, "android")
            }
            AssemblyKind::Core => unreachable!("parse above rejects core"),
        };

    // `scaffold(run_build=true)` returns `Err(Verify)` on the first
    // failing step, so if we're here every build step passed. We still
    // compute `passed` from the step vector (rather than hard-coding
    // `true`) so a future refactor that returns partial failure via
    // `Ok` without changing this call site is caught.
    let passed = build_steps.iter().all(|s| s.passed);

    let mut assembly = serde_json::Map::new();
    assembly.insert(
        "status".to_string(),
        serde_json::Value::String("created".into()),
    );
    assembly.insert(
        "files".to_string(),
        serde_json::Value::Array(files.into_iter().map(serde_json::Value::String).collect()),
    );
    assembly.insert(
        "build_steps".to_string(),
        serde_json::to_value(&build_steps).map_err(|e| VectisError::Internal {
            message: format!("failed to serialize build steps: {e}"),
        })?,
    );

    let value = serde_json::json!({
        "app_name": parsed.app_name,
        "project_dir": project_dir.display().to_string(),
        "platform": written_shell,
        "source": "app.rs",
        "detected_capabilities": parsed
            .capabilities
            .iter()
            .map(|c| c.marker_tag())
            .collect::<Vec<_>>(),
        "unrecognized_capabilities": parsed.unrecognized_capabilities,
        "assembly": assembly,
        "passed": passed,
    });

    Ok(CommandOutcome::Success(value))
}

fn shell_dir_name(shell: AssemblyKind) -> &'static str {
    match shell {
        AssemblyKind::Ios => "iOS",
        AssemblyKind::Android => "Android",
        AssemblyKind::Core => unreachable!("core is not a shell"),
    }
}

fn resolve_project_dir(dir: Option<&Path>) -> Result<PathBuf, VectisError> {
    match dir {
        Some(p) => Ok(p.to_path_buf()),
        None => std::env::current_dir().map_err(VectisError::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "vectis-add-shell-{label}-{}-{nanos}-{n}",
            std::process::id(),
        ))
    }

    #[test]
    fn rejects_unknown_platform() {
        let args = AddShellArgs {
            platform: "windows".into(),
            dir: None,
            android_package: None,
            version_file: None,
        };
        let err = run(&args).expect_err("unknown platform must fail");
        match err {
            VectisError::InvalidProject { message } => {
                assert!(message.contains("windows"), "{message}");
                assert!(message.contains("ios"), "{message}");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn rejects_when_no_core_present() {
        let dir = scratch_dir("no-core");
        std::fs::create_dir_all(&dir).unwrap();
        let args = AddShellArgs {
            platform: "ios".into(),
            dir: Some(dir.clone()),
            android_package: None,
            version_file: None,
        };
        let err = run(&args).expect_err("missing core must fail");
        match err {
            VectisError::InvalidProject { message } => {
                assert!(message.contains("no core assembly found"), "{message}");
                assert!(message.contains("vectis init"), "{message}");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn rejects_when_shell_already_present() {
        let dir = scratch_dir("shell-exists");
        std::fs::create_dir_all(dir.join("shared/src")).unwrap();
        std::fs::write(
            dir.join("shared/src/app.rs"),
            "impl App for Counter {}\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("iOS")).unwrap();

        let args = AddShellArgs {
            platform: "ios".into(),
            dir: Some(dir.clone()),
            android_package: None,
            version_file: None,
        };
        let err = run(&args).expect_err("existing shell must fail");
        match err {
            VectisError::InvalidProject { message } => {
                assert!(message.contains("shell already present"), "{message}");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
}

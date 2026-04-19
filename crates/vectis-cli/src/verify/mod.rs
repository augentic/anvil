//! `vectis verify` -- run the per-assembly compilation pipelines.
//!
//! Chunk 9 lands the full dispatcher: auto-detect which assemblies exist
//! (core always; ios if `iOS/` is present; android if `Android/` is
//! present), check the scoped prerequisites, resolve version pins, run
//! the per-assembly pipelines, and emit the RFC's structured JSON. Each
//! assembly runs independently so a broken iOS build does not mask an
//! otherwise-healthy Android build.
//!
//! The per-assembly pipelines live in sibling modules (`verify::core`,
//! `verify::ios`, `verify::android`) and are reused by `init` for the
//! post-scaffold smoke build -- this keeps the step list / `BuildStep`
//! shape in one place so it cannot drift between the two entry points.

pub mod android;
pub mod core;
pub mod ios;
pub mod pipeline;

use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    CommandOutcome, VerifyArgs,
    error::VectisError,
    prerequisites::{self, AssemblyKind},
    versions::Versions,
};

pub fn run(args: &VerifyArgs) -> Result<CommandOutcome, VectisError> {
    let project_dir = args
        .dir
        .clone()
        .map(Ok)
        .unwrap_or_else(std::env::current_dir)?;

    let mut assemblies = vec![AssemblyKind::Core];
    if project_dir.join("iOS").is_dir() {
        assemblies.push(AssemblyKind::Ios);
    }
    if project_dir.join("Android").is_dir() {
        assemblies.push(AssemblyKind::Android);
    }

    prerequisites::check(&assemblies)?;

    // `--version-file` is validated here even though verify does not
    // consume any pin value today (it does not render templates). This
    // preserves the resolution-override semantics documented on the
    // flag: a bad path must fail early with a structured error rather
    // than be silently ignored.
    let _versions = Versions::resolve(&project_dir, args.version_file.as_deref())?;

    let cache = VerifyCache::new(std::process::id())?;

    let mut assemblies_json = serde_json::Map::new();
    let mut overall_passed = true;

    // Core is always detected.
    let core_steps = core::run_pipeline(&project_dir, cache.swift_dir(), cache.kotlin_dir())?;
    let core_passed = core_steps.iter().all(|s| s.passed);
    overall_passed &= core_passed;
    assemblies_json.insert(
        "core".to_string(),
        serde_json::json!({
            "passed": core_passed,
            "steps": core_steps,
        }),
    );

    if assemblies.contains(&AssemblyKind::Ios) {
        let ios_steps = ios::run_pipeline(&project_dir.join("iOS"), true)?;
        let ios_passed = ios_steps.iter().all(|s| s.passed);
        overall_passed &= ios_passed;
        assemblies_json.insert(
            "ios".to_string(),
            serde_json::json!({
                "passed": ios_passed,
                "steps": ios_steps,
            }),
        );
    }

    if assemblies.contains(&AssemblyKind::Android) {
        let android_steps = android::run_pipeline(&project_dir.join("Android"))?;
        let android_passed = android_steps.iter().all(|s| s.passed);
        overall_passed &= android_passed;
        assemblies_json.insert(
            "android".to_string(),
            serde_json::json!({
                "passed": android_passed,
                "steps": android_steps,
            }),
        );
    }

    let value = serde_json::json!({
        "project_dir": project_dir.display().to_string(),
        "passed": overall_passed,
        "assemblies": assemblies_json,
    });

    // Explicitly drop the cache so the scratch directory is cleaned up
    // whether or not the pipelines passed. Propagated pipeline errors
    // (missing binary, etc.) short-circuit before here; on that path
    // Rust's drop glue still runs the cleanup.
    drop(cache);

    Ok(CommandOutcome::Success(value))
}

/// Scratch directory for core-pipeline codegen output.
///
/// Writes to `$HOME/.cache/vectis/verify-<pid>/{swift,kotlin}` when
/// `$HOME` is available; otherwise falls back to
/// `<std::env::temp_dir()>/vectis-verify-<pid>/`. We prefer the cache
/// directory because macOS aggressively rotates `/tmp` and we have
/// observed multi-minute `cargo` invocations outlive the directory
/// they were launched from (chunks 6/7/8 all hit this).
///
/// `Drop` removes the directory tree best-effort; a leftover scratch
/// dir is harmless.
struct VerifyCache {
    root: PathBuf,
    swift: PathBuf,
    kotlin: PathBuf,
}

impl VerifyCache {
    fn new(pid: u32) -> Result<Self, VectisError> {
        let root = match std::env::var_os("HOME") {
            Some(home) => PathBuf::from(home)
                .join(".cache")
                .join("vectis")
                .join(format!("verify-{pid}")),
            None => std::env::temp_dir().join(format!("vectis-verify-{pid}")),
        };
        // Best-effort cleanup of any leftover from a prior run (same
        // pid -- unlikely but possible across forks).
        let _ = fs::remove_dir_all(&root);
        let swift = root.join("swift");
        let kotlin = root.join("kotlin");
        fs::create_dir_all(&swift)?;
        fs::create_dir_all(&kotlin)?;
        Ok(Self {
            root,
            swift,
            kotlin,
        })
    }

    fn swift_dir(&self) -> &Path {
        &self.swift
    }

    fn kotlin_dir(&self) -> &Path {
        &self.kotlin
    }
}

impl Drop for VerifyCache {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

//! Core-assembly verify pipeline (RFC-6 § Verify Pipeline § Core).
//!
//! Seven fixed steps, run from the project root, stopping at the first
//! failure:
//!
//! 1. `cargo check`
//! 2. `cargo clippy --all-targets -- -D warnings`
//! 3. `cargo deny check`
//! 4. `cargo vet`
//! 5. `cargo run -p shared --bin codegen --features codegen,facet_typegen -- --language swift --output-dir <cache>/swift`
//! 6. `cargo build -p shared --features uniffi` (produce the cdylib
//!    artefact uniffi's `generate_bindings` requires -- see the
//!    rationale comment on the step itself)
//! 7. `cargo run -p shared --bin codegen --features codegen,facet_typegen -- --language kotlin --output-dir <cache>/kotlin`
//!
//! The codegen output directories are scratch paths supplied by the
//! caller -- verify owns their lifecycle (create before, remove after).
//! The caller picks `$HOME/.cache/vectis/verify-<pid>/...` over `/tmp`
//! because macOS rotates `/tmp` aggressively; see `verify::run` for
//! the lifecycle.
//!
//! ## First-run supply-chain bootstrap
//!
//! `cargo vet` on a freshly scaffolded project fails immediately -- the
//! template ships an empty `supply-chain/audits.toml` and no
//! `[[exemptions.*]]` blocks in `config.toml`, which means every
//! transitive crate is unaudited. Before the `cargo vet` step we detect
//! this "fresh scaffold" state (empty imports.lock and no exemptions
//! in config.toml) and run `cargo vet regenerate exemptions` once to
//! seed a baseline. Subsequent verifies skip the regen and run vet
//! directly, so real supply-chain drift (new untrusted transitive dep)
//! still surfaces as a failure rather than being silently exempted.

use std::path::Path;
use std::process::Command;

use crate::error::VectisError;
use crate::verify::pipeline::{BuildStep, run_step};

/// Run the core verify pipeline under `project_dir`.
///
/// `codegen_swift_dir` / `codegen_kotlin_dir` are the scratch output
/// paths for steps 5 and 6; the caller is responsible for their
/// lifecycle. The function stops at the first failing step (later steps
/// depend on the earlier ones; re-running them against a known-broken
/// tree only muddies the output).
pub fn run_pipeline(
    project_dir: &Path,
    codegen_swift_dir: &Path,
    codegen_kotlin_dir: &Path,
) -> Result<Vec<BuildStep>, VectisError> {
    let mut steps: Vec<BuildStep> = Vec::with_capacity(7);

    let check = run_step(
        "cargo check",
        Command::new("cargo").arg("check").current_dir(project_dir),
    )?;
    let check_passed = check.passed;
    steps.push(check);
    if !check_passed {
        return Ok(steps);
    }

    let clippy = run_step(
        "cargo clippy",
        Command::new("cargo")
            .args(["clippy", "--all-targets", "--", "-D", "warnings"])
            .current_dir(project_dir),
    )?;
    let clippy_passed = clippy.passed;
    steps.push(clippy);
    if !clippy_passed {
        return Ok(steps);
    }

    let deny = run_step(
        "cargo deny",
        Command::new("cargo")
            .args(["deny", "check"])
            .current_dir(project_dir),
    )?;
    let deny_passed = deny.passed;
    steps.push(deny);
    if !deny_passed {
        return Ok(steps);
    }

    // On a freshly scaffolded project the supply-chain files are empty
    // -- `cargo vet` would fail before the user has a chance to
    // certify anything. Detect the pristine state and seed exemptions
    // once; on subsequent verifies the regen is skipped so real drift
    // is not silently exempted.
    if needs_supply_chain_bootstrap(project_dir) {
        let prep = run_step(
            "cargo vet",
            Command::new("cargo")
                .args(["vet", "regenerate", "exemptions"])
                .current_dir(project_dir),
        )?;
        let prep_passed = prep.passed;
        if !prep_passed {
            steps.push(prep);
            return Ok(steps);
        }
    }

    let vet = run_step(
        "cargo vet",
        Command::new("cargo").arg("vet").current_dir(project_dir),
    )?;
    let vet_passed = vet.passed;
    steps.push(vet);
    if !vet_passed {
        return Ok(steps);
    }

    let swift = run_step(
        "codegen swift",
        Command::new("cargo")
            .args([
                "run",
                "-p",
                "shared",
                "--bin",
                "codegen",
                "--features",
                "codegen,facet_typegen",
                "--",
                "--language",
                "swift",
                "--output-dir",
            ])
            .arg(codegen_swift_dir)
            .current_dir(project_dir),
    )?;
    let swift_passed = swift.passed;
    steps.push(swift);
    if !swift_passed {
        return Ok(steps);
    }

    // Build the shared cdylib so that the Kotlin codegen step (which
    // runs uniffi's `generate_bindings` helper) can locate a
    // `libshared.dylib` / `.so` to introspect. `cargo run --bin codegen`
    // only builds the binary + its direct rlib dep on `shared`; it does
    // NOT produce the cdylib artefact uniffi looks for at
    // `target/debug/libshared.{dylib,so}`, so running codegen kotlin
    // against a fresh scaffold fails with "library ... not found".
    // Chunk 9's happy-path test scaffolded `--shells android` first,
    // which pre-built the cdylib via rust-android-gradle; chunk-10
    // add-shell flows (init core-only, then add-shell ios/android) hit
    // the failure directly. Pre-building here makes verify hermetic
    // regardless of which shells exist on disk.
    let cdylib = run_step(
        "cargo build shared cdylib",
        Command::new("cargo")
            .args(["build", "-p", "shared", "--features", "uniffi"])
            .current_dir(project_dir),
    )?;
    let cdylib_passed = cdylib.passed;
    steps.push(cdylib);
    if !cdylib_passed {
        return Ok(steps);
    }

    let kotlin = run_step(
        "codegen kotlin",
        Command::new("cargo")
            .args([
                "run",
                "-p",
                "shared",
                "--bin",
                "codegen",
                "--features",
                "codegen,facet_typegen",
                "--",
                "--language",
                "kotlin",
                "--output-dir",
            ])
            .arg(codegen_kotlin_dir)
            .current_dir(project_dir),
    )?;
    steps.push(kotlin);

    Ok(steps)
}

/// Detect the "fresh scaffold" state where `cargo vet` would fail with
/// zero audited crates. We look for two signals and require both:
///
/// 1. `supply-chain/imports.lock` is empty / header-only (the file
///    shipped by the template is just a `# Auto-managed...` comment).
/// 2. `supply-chain/config.toml` contains no `[[exemptions.*]]` block.
///
/// If either signal is missing (imports.lock has real content OR the
/// user has curated exemptions) we leave supply chain alone.
fn needs_supply_chain_bootstrap(project_dir: &Path) -> bool {
    let imports_path = project_dir.join("supply-chain").join("imports.lock");
    let config_path = project_dir.join("supply-chain").join("config.toml");

    if !imports_path.exists() || !config_path.exists() {
        // Project wasn't scaffolded by vectis, or supply-chain was
        // removed deliberately -- don't surprise-regenerate.
        return false;
    }

    let imports = std::fs::read_to_string(&imports_path).unwrap_or_default();
    let imports_effective: String = imports
        .lines()
        .filter(|line| {
            let t = line.trim();
            !t.is_empty() && !t.starts_with('#')
        })
        .collect();
    let imports_fresh = imports_effective.trim().is_empty();

    let config = std::fs::read_to_string(&config_path).unwrap_or_default();
    let config_has_exemptions = config.contains("[[exemptions.");

    imports_fresh && !config_has_exemptions
}

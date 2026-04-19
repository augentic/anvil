//! Android-assembly verify pipeline (RFC-6 § Verify Pipeline § Android).
//!
//! Two fixed steps, run from `Android/`, stopping at the first failure:
//!
//! 1. `make build` — generate Kotlin types via codegen
//! 2. `./gradlew :app:assembleDebug` — build the APK
//!
//! The RFC enumerates five steps (wrapper bootstrap, `local.properties`,
//! `make build`, `:shared:cargoBuild`, `:app:assembleDebug`) but the
//! wrapper + `local.properties` are scaffold-time concerns owned by
//! `init::android::scaffold`, and `:app:assembleDebug` depends on
//! `:shared:cargoBuild` so invoking the former exercises both. `init`
//! and `add-shell` write the wrapper once; verify assumes it exists and
//! re-bootstrapping it would clobber any hand-edited
//! `gradle-wrapper.properties` (chunk-9 Notes column).

use std::path::Path;
use std::process::Command;

use crate::error::VectisError;
use crate::verify::pipeline::{BuildStep, run_step};

/// Run the Android verify pipeline under `android_root`.
pub fn run_pipeline(android_root: &Path) -> Result<Vec<BuildStep>, VectisError> {
    let mut steps = Vec::with_capacity(2);

    let make = run_step(
        "make build",
        Command::new("make").arg("build").current_dir(android_root),
    )?;
    let make_passed = make.passed;
    steps.push(make);
    if !make_passed {
        return Ok(steps);
    }

    let assemble = run_step(
        "gradlew assembleDebug",
        Command::new("./gradlew")
            .arg(":app:assembleDebug")
            .current_dir(android_root),
    )?;
    steps.push(assemble);

    Ok(steps)
}

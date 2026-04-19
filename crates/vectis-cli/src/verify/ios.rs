//! iOS-assembly pipeline (RFC-6 § Verify Pipeline § iOS).
//!
//! Four fixed steps, run from `iOS/`, stopping at the first failure:
//!
//! 1. `make typegen` — generate the `SharedTypes` Swift package
//! 2. `make package` — build the `Shared` UniFFI Swift package via `cargo swift`
//! 3. `make xcode` — generate the Xcode project via `xcodegen`
//! 4. `xcodebuild build` — simulator build of the generated project
//!
//! `init::ios::scaffold` reuses this module for the first three steps
//! (post-scaffold smoke build). `verify::run` calls the full four-step
//! pipeline. The `xcodebuild` step targets
//! `-destination 'generic/platform=iOS Simulator'` so it is hermetic
//! across Xcode versions -- chunk-7's `name=iPhone 15` pin rotted when
//! Xcode 26 stopped shipping the iPhone 15 runtime by default (see the
//! chunk-7 Notes column for the full story).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::VectisError;
use crate::verify::pipeline::{BuildStep, run_step};

/// Run the iOS pipeline under `ios_root`.
///
/// When `include_xcodebuild` is `true` the 4th step (`xcodebuild build`)
/// runs after the `make` chain; when `false` the function stops after
/// `make xcode`. init uses `false` (the Xcode project is proof enough
/// that the templates rendered coherently); verify uses `true`.
pub fn run_pipeline(
    ios_root: &Path,
    include_xcodebuild: bool,
) -> Result<Vec<BuildStep>, VectisError> {
    let mut steps = Vec::with_capacity(if include_xcodebuild { 4 } else { 3 });

    for name in ["make typegen", "make package", "make xcode"] {
        let target = &name["make ".len()..];
        let step = run_step(
            name,
            Command::new("make").arg(target).current_dir(ios_root),
        )?;
        let passed = step.passed;
        steps.push(step);
        if !passed {
            return Ok(steps);
        }
    }

    if include_xcodebuild {
        let (scheme, project_path) = find_xcodeproj(ios_root)?;
        let step = run_step(
            "xcodebuild",
            Command::new("xcodebuild")
                .args([
                    "build",
                    "-project",
                    project_path.to_string_lossy().as_ref(),
                    "-scheme",
                    &scheme,
                    "-destination",
                    "generic/platform=iOS Simulator",
                    "-configuration",
                    "Debug",
                    "CODE_SIGNING_ALLOWED=NO",
                ])
                .current_dir(ios_root),
        )?;
        steps.push(step);
    }

    Ok(steps)
}

/// Find the generated `.xcodeproj` under `ios_root` and derive a scheme
/// name from its stem.
///
/// xcodegen produces exactly one `.xcodeproj` per project.yml; if we see
/// zero or more than one we return `Verify` with a clear message rather
/// than pick arbitrarily. Agents re-running verify after deleting the
/// project will see this error and know to re-run `make xcode`.
fn find_xcodeproj(ios_root: &Path) -> Result<(String, PathBuf), VectisError> {
    let mut found: Vec<PathBuf> = fs::read_dir(ios_root)
        .map_err(VectisError::from)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("xcodeproj"))
        .collect();
    if found.is_empty() {
        return Err(VectisError::Verify {
            message: format!(
                "no .xcodeproj found under {} -- run `make xcode` first",
                ios_root.display()
            ),
        });
    }
    if found.len() > 1 {
        return Err(VectisError::Verify {
            message: format!(
                "multiple .xcodeproj bundles found under {}; expected exactly one",
                ios_root.display()
            ),
        });
    }
    let path = found.pop().expect("non-empty by check above");
    let scheme = path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| VectisError::Verify {
            message: format!("could not derive scheme from {}", path.display()),
        })?
        .to_string();
    Ok((scheme, path))
}

//! `--verify` scaffold-and-build loop for `vectis update-versions`.
//!
//! For each capability combination in [`CAP_MATRIX`], we:
//!
//! 1. create a scratch directory,
//! 2. write the proposed pins out as a `versions.toml` inside it,
//! 3. call `init::run` (core-only) pointed at that pins file,
//! 4. call `verify::run` on the scaffolded project,
//! 5. best-effort clean up the scratch dir.
//!
//! Every combo runs even if a prior one fails -- the caller reports the
//! full matrix so the user can see how many combos survived the bump.
//! `passed` is `all combos passed`.
//!
//! The matrix is deliberately core-only. The RFC's `update-versions
//! --verify` spec talks about cap combinations, not shell combinations;
//! driving iOS / Android in this loop would multiply cost by three for
//! no additional coverage of the pin set (the shells vendor the pins
//! the Crux core produces).

use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    InitArgs, VerifyArgs,
    error::VectisError,
    init, verify,
    versions::Versions,
};

/// The fixed cap matrix the `--verify` gate exercises.
///
/// Each entry is the `--caps` string `vectis init` receives. The empty
/// string is render-only (no capability crates pulled in). The "full"
/// row exercises every cap-conditional template region in one pass.
pub const CAP_MATRIX: &[&str] = &[
    "",                          // render-only
    "http",                      // http-only
    "http,kv",                   // multi-cap
    "http,kv,time,platform,sse", // full
];

/// Outcome of one matrix entry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ComboResult {
    pub caps: String,
    pub passed: bool,
    /// Verify JSON (the full `{project_dir, passed, assemblies}` shape)
    /// when we got far enough to run verify; `None` if init bailed
    /// before verify could run (in which case `error` carries the
    /// reason).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verify: Option<serde_json::Value>,
    /// Short diagnostic string set on init failure or when verify
    /// itself returned a structured error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Verification summary returned to the orchestrator.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MatrixResult {
    pub passed: bool,
    pub combos: Vec<ComboResult>,
}

/// Run the cap matrix against `proposed` pins. `scratch_root` is the
/// directory under which per-combo project dirs will be created; the
/// caller owns its lifecycle. We still clean up the per-combo dirs as
/// we go so a partial failure does not leak the full matrix to disk.
pub fn run(proposed: &Versions, scratch_root: &Path) -> Result<MatrixResult, VectisError> {
    fs::create_dir_all(scratch_root)?;
    let pins_str = render_pins_toml(proposed);

    let mut combos: Vec<ComboResult> = Vec::with_capacity(CAP_MATRIX.len());
    for (idx, caps) in CAP_MATRIX.iter().enumerate() {
        let combo_dir = scratch_root.join(format!("combo-{idx}"));
        // Best-effort wipe in case a prior run left something behind.
        let _ = fs::remove_dir_all(&combo_dir);
        fs::create_dir_all(&combo_dir)?;

        let pins_file = combo_dir.join("pins.toml");
        fs::write(&pins_file, &pins_str)?;

        combos.push(run_single_combo(caps, &combo_dir, &pins_file));

        // Clean up after each combo so a full matrix run does not
        // accumulate gigabytes of target/ output on disk.
        let _ = fs::remove_dir_all(&combo_dir);
    }

    let passed = combos.iter().all(|c| c.passed);
    Ok(MatrixResult { passed, combos })
}

/// Scaffold + verify a single combo. Swallows all inner errors into a
/// diagnostic string on [`ComboResult::error`] so the matrix loop can
/// continue with the next combo.
fn run_single_combo(caps: &str, project_dir: &Path, pins_file: &Path) -> ComboResult {
    let init_args = InitArgs {
        app_name: "BumpProbe".to_string(),
        dir: Some(PathBuf::from(project_dir)),
        caps: Some(caps.to_string()),
        shells: Some(String::new()),
        android_package: None,
        version_file: Some(PathBuf::from(pins_file)),
    };

    if let Err(err) = init::run(&init_args) {
        return ComboResult {
            caps: caps.to_string(),
            passed: false,
            verify: None,
            error: Some(format!("init failed: {err}")),
        };
    }

    let verify_args = VerifyArgs {
        dir: Some(PathBuf::from(project_dir)),
        version_file: Some(PathBuf::from(pins_file)),
    };

    match verify::run(&verify_args) {
        Ok(crate::CommandOutcome::Success(value)) => {
            let combo_passed = value
                .get("passed")
                .and_then(|p| p.as_bool())
                .unwrap_or(false);
            ComboResult {
                caps: caps.to_string(),
                passed: combo_passed,
                verify: Some(value),
                error: None,
            }
        }
        Ok(crate::CommandOutcome::Stub { .. }) => ComboResult {
            caps: caps.to_string(),
            passed: false,
            verify: None,
            error: Some("verify returned an unexpected Stub outcome".to_string()),
        },
        Err(err) => ComboResult {
            caps: caps.to_string(),
            passed: false,
            verify: None,
            error: Some(format!("verify failed: {err}")),
        },
    }
}

/// Serialise a [`Versions`] back to TOML in the same shape the embedded
/// defaults use. Kept local to this module rather than on
/// `versions.rs` because the write shape is update-versions' concern:
/// comments, ordering, and any header text live in the orchestrator,
/// not in the resolver.
pub fn render_pins_toml(v: &Versions) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    writeln!(&mut out, "[crux]").unwrap();
    writeln!(&mut out, "crux_core     = \"{}\"", v.crux.crux_core).unwrap();
    writeln!(&mut out, "crux_http     = \"{}\"", v.crux.crux_http).unwrap();
    writeln!(&mut out, "crux_kv       = \"{}\"", v.crux.crux_kv).unwrap();
    writeln!(&mut out, "crux_time     = \"{}\"", v.crux.crux_time).unwrap();
    writeln!(&mut out, "crux_platform = \"{}\"", v.crux.crux_platform).unwrap();
    writeln!(&mut out, "facet           = \"{}\"", v.crux.facet).unwrap();
    writeln!(
        &mut out,
        "facet_generate  = \"{}\"",
        v.crux.facet_generate
    )
    .unwrap();
    writeln!(&mut out, "serde           = \"{}\"", v.crux.serde).unwrap();
    writeln!(&mut out, "serde_json      = \"{}\"", v.crux.serde_json).unwrap();
    writeln!(&mut out, "uniffi      = \"{}\"", v.crux.uniffi).unwrap();
    writeln!(&mut out, "cargo_swift = \"{}\"", v.crux.cargo_swift).unwrap();
    writeln!(&mut out).unwrap();

    writeln!(&mut out, "[android]").unwrap();
    writeln!(&mut out, "compose_bom = \"{}\"", v.android.compose_bom).unwrap();
    writeln!(&mut out, "koin        = \"{}\"", v.android.koin).unwrap();
    writeln!(&mut out, "ktor        = \"{}\"", v.android.ktor).unwrap();
    writeln!(&mut out, "kotlin      = \"{}\"", v.android.kotlin).unwrap();
    writeln!(&mut out, "agp         = \"{}\"", v.android.agp).unwrap();
    writeln!(&mut out, "gradle      = \"{}\"", v.android.gradle).unwrap();
    if let Some(ndk) = &v.android.ndk {
        writeln!(&mut out, "ndk         = \"{ndk}\"").unwrap();
    }
    writeln!(&mut out).unwrap();

    writeln!(&mut out, "[ios]").unwrap();
    writeln!(&mut out).unwrap();

    writeln!(&mut out, "[tooling]").unwrap();
    writeln!(&mut out, "cargo_deny = \"{}\"", v.tooling.cargo_deny).unwrap();
    writeln!(&mut out, "cargo_vet  = \"{}\"", v.tooling.cargo_vet).unwrap();
    writeln!(&mut out, "xcodegen   = \"{}\"", v.tooling.xcodegen).unwrap();

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_pins_toml_round_trips_through_versions_parser() {
        let baseline = Versions::embedded().expect("embedded must parse");
        let rendered = render_pins_toml(&baseline);
        let reparsed: Versions =
            toml::from_str(&rendered).expect("rendered TOML must parse back into Versions");
        assert_eq!(baseline, reparsed);
    }

    #[test]
    fn render_pins_toml_round_trips_with_ndk_set() {
        let mut v = Versions::embedded().unwrap();
        v.android.ndk = Some("27.1.12297006".to_string());
        let rendered = render_pins_toml(&v);
        assert!(
            rendered.contains("ndk         = \"27.1.12297006\""),
            "ndk line missing: {rendered}"
        );
        let reparsed: Versions = toml::from_str(&rendered).unwrap();
        assert_eq!(reparsed.android.ndk.as_deref(), Some("27.1.12297006"));
    }

    #[test]
    fn cap_matrix_is_non_empty_and_contains_the_empty_and_full_rows() {
        // Sanity: every combo exercises a distinct cap shape of the
        // render-only baseline through the full cap surface. The exact
        // set is project policy; the presence of the endpoints is a
        // regression gate.
        assert!(CAP_MATRIX.contains(&""), "render-only row missing");
        assert!(
            CAP_MATRIX
                .iter()
                .any(|c| c.contains("sse") && c.contains("platform")),
            "full-cap row missing"
        );
    }
}

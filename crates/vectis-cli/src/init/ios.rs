//! iOS assembly scaffolding for `vectis init --shells ios`.
//!
//! Chunk 7 lands the iOS shell: the embedded chunk-3b templates are rendered
//! into `iOS/<AppName>/...` and, when prerequisites are present, the
//! generated `iOS/Makefile` is invoked to produce the `SharedTypes` /
//! `Shared` Swift packages plus the Xcode project. The handler refuses to
//! overwrite an existing `iOS/` directory so re-running on a project that
//! already has the shell fails fast.
//!
//! Atomic refusal mirrors `init::core::scaffold`: every target path is
//! computed (with placeholder substitution applied to directory and
//! file-name segments) and checked for prior existence *before* any
//! directory is created or any byte is written.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::VectisError;
use crate::templates::{Capability, Params, ios, render, substitute_path};
use crate::verify;
pub use crate::verify::pipeline::BuildStep;

/// Result of a successful iOS scaffold.
///
/// `files` lists target paths in the order they were written -- which is
/// the order the embedded `TEMPLATES` slice declares (matches the chunk-3b
/// MANIFEST § Path mapping order).
#[derive(Debug)]
pub struct IosScaffold {
    pub files: Vec<String>,
    /// Build steps that ran and their per-step pass/fail status. When
    /// `make` is skipped (e.g. prerequisites missing under `--no-build`)
    /// the vector is empty.
    pub build_steps: Vec<BuildStep>,
}

/// Render and write the iOS templates under `project_dir`, then run the
/// chunk-3b `iOS/Makefile` pipeline (`typegen`, `package`, `xcode`).
///
/// `caps` selects which capability-marked regions of the iOS templates are
/// kept. Today only `Core.swift` carries CAP markers (`http`, `kv`, `time`,
/// `platform`); future Sse work would extend this.
///
/// On any prior `iOS/` directory or pre-existing target file the function
/// returns `InvalidProject` *before* writing anything. The build pipeline
/// is invoked from `project_dir/iOS/`; non-zero exit from any step yields
/// `Verify` so the caller can splice the failure into the structured JSON
/// output. The pipeline can be skipped for unit tests via `run_build`.
pub fn scaffold(
    project_dir: &Path,
    app_name: &str,
    caps: &[Capability],
    params: &Params,
    run_build: bool,
) -> Result<IosScaffold, VectisError> {
    let ios_root = project_dir.join("iOS");
    if ios_root.exists() {
        return Err(VectisError::InvalidProject {
            message: format!(
                "refusing to overwrite existing iOS shell at {} (delete it first or use `vectis add-shell ios`)",
                ios_root.display()
            ),
        });
    }

    let mut planned: Vec<(PathBuf, String, String)> = Vec::with_capacity(ios::TEMPLATES.len());
    for entry in ios::TEMPLATES {
        let rendered_target = substitute_path(entry.target, params);
        let target = project_dir.join(&rendered_target);
        if target.exists() {
            return Err(VectisError::InvalidProject {
                message: format!(
                    "refusing to overwrite existing file at {} (run `vectis init` against an empty directory)",
                    target.display()
                ),
            });
        }
        let rendered = render(entry.contents, params, caps);
        planned.push((target, rendered_target, rendered));
    }

    let mut written = Vec::with_capacity(planned.len());
    for (path, target_str, contents) in planned {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents)?;
        written.push(target_str);
    }

    // The iOS shell uses `Info.plist` keys generated inline by xcodegen
    // from `project.yml`; chunk 7 deliberately does not pre-write a
    // `Info.plist`. xcodegen will materialize one at build time.
    let _ = app_name;

    let build_steps = if run_build {
        let steps = verify::ios::run_pipeline(&ios_root, false)?;
        if let Some(failing) = steps.iter().find(|s| !s.passed) {
            // init's contract is "scaffold succeeds or we error out so
            // the user fixes their toolchain". Per-step detail is lost
            // on the Err path (matches chunk-7 behaviour); verify's
            // flow keeps the full step vector intact for the user.
            return Err(VectisError::Verify {
                message: format!("iOS build step `{}` failed", failing.name),
            });
        }
        steps
    } else {
        Vec::new()
    };

    Ok(IosScaffold {
        files: written,
        build_steps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::versions::Versions;
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
            "vectis-init-ios-{label}-{}-{nanos}-{n}",
            std::process::id(),
        ))
    }

    fn sample_params() -> Params {
        let v = Versions::embedded().expect("embedded defaults must parse");
        Params {
            app_name: "Counter".into(),
            app_struct: "Counter".into(),
            app_name_lower: "counter".into(),
            android_package: "com.vectis.counter".into(),
            crux_core_version: v.crux.crux_core,
            crux_http_version: v.crux.crux_http,
            crux_kv_version: v.crux.crux_kv,
            crux_time_version: v.crux.crux_time,
            crux_platform_version: v.crux.crux_platform,
            facet_version: v.crux.facet,
            serde_version: v.crux.serde,
            uniffi_version: v.crux.uniffi,
            agp_version: v.android.agp,
            kotlin_version: v.android.kotlin,
            compose_bom_version: v.android.compose_bom,
            ktor_version: v.android.ktor,
            koin_version: v.android.koin,
            android_ndk_version: "__ANDROID_NDK_VERSION__".into(),
        }
    }

    #[test]
    fn scaffold_writes_every_ios_file_with_substituted_paths() {
        let dir = scratch_dir("substitute");
        fs::create_dir_all(&dir).unwrap();
        let result = scaffold(&dir, "Counter", &[], &sample_params(), false)
            .expect("scaffold must succeed");
        assert_eq!(result.files.len(), ios::TEMPLATES.len());
        // Spot-check the substituted paths.
        assert!(dir.join("iOS/project.yml").is_file());
        assert!(dir.join("iOS/Makefile").is_file());
        assert!(dir.join("iOS/Counter/CounterApp.swift").is_file());
        assert!(dir.join("iOS/Counter/Core.swift").is_file());
        assert!(dir.join("iOS/Counter/ContentView.swift").is_file());
        assert!(dir.join("iOS/Counter/Views/LoadingScreen.swift").is_file());
        assert!(dir.join("iOS/Counter/Views/HomeScreen.swift").is_file());
        // No build steps when run_build=false.
        assert!(result.build_steps.is_empty());
    }

    #[test]
    fn scaffold_substitutes_app_name_in_file_contents() {
        let dir = scratch_dir("contents");
        fs::create_dir_all(&dir).unwrap();
        scaffold(&dir, "Counter", &[], &sample_params(), false).unwrap();

        let app_swift = fs::read_to_string(dir.join("iOS/Counter/CounterApp.swift")).unwrap();
        assert!(
            !app_swift.contains("__APP_NAME__"),
            "placeholder still present in CounterApp.swift"
        );
        assert!(
            app_swift.contains("struct CounterApp: App"),
            "expected struct CounterApp in CounterApp.swift"
        );

        let project_yml = fs::read_to_string(dir.join("iOS/project.yml")).unwrap();
        assert!(!project_yml.contains("__APP_NAME__"));
        assert!(!project_yml.contains("__APP_NAME_LOWER__"));
        assert!(project_yml.contains("name: Counter"));
        assert!(project_yml.contains("com.vectis.counter"));
    }

    #[test]
    fn scaffold_strips_cap_blocks_for_render_only() {
        let dir = scratch_dir("render-only");
        fs::create_dir_all(&dir).unwrap();
        scaffold(&dir, "Counter", &[], &sample_params(), false).unwrap();

        let core_swift = fs::read_to_string(dir.join("iOS/Counter/Core.swift")).unwrap();
        assert!(
            !core_swift.contains("<<<CAP:"),
            "leftover open marker in Core.swift"
        );
        // Render-only: no cap-specific case arms should remain.
        assert!(!core_swift.contains("performHttpRequest"));
        assert!(!core_swift.contains("case .http"));
        assert!(!core_swift.contains("case .keyValue"));
        assert!(!core_swift.contains("case .time"));
        assert!(!core_swift.contains("case .platform"));
    }

    #[test]
    fn scaffold_includes_selected_cap_blocks() {
        let dir = scratch_dir("with-http");
        fs::create_dir_all(&dir).unwrap();
        scaffold(&dir, "Counter", &[Capability::Http], &sample_params(), false).unwrap();

        let core_swift = fs::read_to_string(dir.join("iOS/Counter/Core.swift")).unwrap();
        assert!(!core_swift.contains("<<<CAP:"));
        // Both the case arm and the helper function must land together
        // (chunk-3b MANIFEST § Cap-marker reference -- Swift exhaustive
        // switch).
        assert!(core_swift.contains("case .http"));
        assert!(core_swift.contains("performHttpRequest"));
        // KV is unselected -- its arm must not leak.
        assert!(!core_swift.contains("case .keyValue"));
    }

    #[test]
    fn scaffold_refuses_to_overwrite_existing_ios_dir() {
        let dir = scratch_dir("no-overwrite");
        fs::create_dir_all(dir.join("iOS")).unwrap();

        let err = scaffold(&dir, "Counter", &[], &sample_params(), false)
            .expect_err("must refuse to overwrite iOS/");
        match err {
            VectisError::InvalidProject { message } => {
                assert!(message.contains("refusing to overwrite existing iOS shell"));
            }
            other => panic!("unexpected: {other:?}"),
        }
        // No project files should have been written.
        assert!(!dir.join("iOS/project.yml").exists());
    }
}

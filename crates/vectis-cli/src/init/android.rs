//! Android assembly scaffolding for `vectis init --shells android`.
//!
//! Chunk 8 lands the Android shell: the embedded chunk-3c templates are
//! rendered into `Android/...`, the Gradle wrapper is bootstrapped, and a
//! per-developer `local.properties` is written so the project is hermetic
//! from the first command. The handler refuses to overwrite an existing
//! `Android/` directory so re-running on a project that already has the
//! shell fails fast.
//!
//! Atomic refusal mirrors `init::ios::scaffold` and `init::core::scaffold`:
//! every target path is computed (with placeholder substitution applied to
//! directory and file-name segments) and checked for prior existence
//! *before* any directory is created or any byte is written.
//!
//! After files are written the scaffold:
//!
//! 1. Detects the JDK 21 install path on macOS via `/usr/libexec/java_home -v 21`
//!    and appends `org.gradle.java.home=<path>` to `gradle.properties` so
//!    the project doesn't depend on the developer's `JAVA_HOME`.
//! 2. Writes `local.properties` with `sdk.dir=$ANDROID_HOME` so Gradle
//!    finds the SDK without per-developer config.
//! 3. Runs `gradle wrapper --gradle-version <pin>` from `Android/` so the
//!    wrapper exists for the build pipeline. Required because chunk 3c's
//!    `gradle/wrapper/` artefacts are intentionally not templates (they
//!    are produced by the wrapper task, not authored by hand).
//! 4. Runs `make build` (codegen) and `./gradlew :app:assembleDebug` when
//!    `run_build` is true, capturing per-step pass/fail into `BuildStep`s
//!    for the JSON output.
//!
//! NDK detection: chunk 4's `Versions::android.ndk` is `Option<String>`. If
//! the user file or override doesn't pin one (the embedded defaults
//! deliberately don't), the scaffold detects an installed version from
//! `$ANDROID_HOME/ndk/<version>/` -- the highest version, sorted lexically
//! over directory entries, wins. Pinning a version that isn't installed
//! yields a confusing `rust-android-gradle` "NDK not found" error, so
//! detection is preferred.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::VectisError;
use crate::templates::{Capability, Params, android, render, substitute_path_with};
use crate::verify;
use crate::verify::pipeline::{BuildStep, run_step};
use crate::versions::Versions;

/// Result of a successful Android scaffold.
///
/// `files` lists target paths in the order they were written -- which is
/// the order the embedded `TEMPLATES` slice declares (matches the chunk-3c
/// MANIFEST § Path mapping order). Skipped whole-file conditionals are
/// excluded (they were never written).
#[derive(Debug)]
pub struct AndroidScaffold {
    pub files: Vec<String>,
    /// Build steps that ran and their per-step pass/fail status. When
    /// `run_build=false` the vector is empty.
    pub build_steps: Vec<BuildStep>,
}

/// Render and write the Android templates under `project_dir`, then
/// bootstrap the Gradle wrapper, write `local.properties`, and (when
/// `run_build` is true) run the `make build` + `./gradlew
/// :app:assembleDebug` pipeline.
///
/// `caps` selects which capability-marked regions of the Android templates
/// are kept and which whole-file conditionals are included. Today only
/// `Core.kt`, `libs.versions.toml`, `app-build.gradle.kts`, and
/// `AndroidManifest.xml` carry CAP markers; only `network-security-config.xml`
/// is whole-file conditional (on `http` or `sse`).
///
/// On any prior `Android/` directory or pre-existing target file the
/// function returns `InvalidProject` *before* writing anything. The build
/// pipeline runs from `project_dir`; non-zero exit from any step yields
/// `Verify` so the caller can splice the failure into the structured JSON
/// output. The pipeline can be skipped for unit tests via `run_build`.
pub fn scaffold(
    project_dir: &Path,
    android_package: &str,
    caps: &[Capability],
    params: &Params,
    versions: &Versions,
    run_build: bool,
) -> Result<AndroidScaffold, VectisError> {
    let android_root = project_dir.join("Android");
    if android_root.exists() {
        return Err(VectisError::InvalidProject {
            message: format!(
                "refusing to overwrite existing Android shell at {} (delete it first or use `vectis add-shell android`)",
                android_root.display()
            ),
        });
    }

    let android_package_path = android_package_to_path(android_package);

    let mut planned: Vec<(PathBuf, String, String)> = Vec::with_capacity(android::TEMPLATES.len());
    for entry in android::TEMPLATES {
        if !entry.include_when.should_include(caps) {
            continue;
        }
        let rendered_target =
            substitute_path_with(entry.target, params, Some(&android_package_path));
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

    // Hermetic post-processing: write `org.gradle.java.home` (when we can
    // detect a JDK 21) and `local.properties` (always, when ANDROID_HOME is
    // set -- it's a prereq so this is a hard expectation).
    write_java_home(&android_root)?;
    write_local_properties(&android_root)?;

    let build_steps = if run_build {
        run_pipeline(project_dir, &android_root, versions, caps)?
    } else {
        Vec::new()
    };

    Ok(AndroidScaffold {
        files: written,
        build_steps,
    })
}

/// Translate an Android package (`com.vectis.counter`) into its on-disk
/// path segment (`com/vectis/counter`). The chunk-3c MANIFEST documents
/// this as a derived placeholder -- it never appears in file contents,
/// only in target paths under `Android/app/src/main/java/...`.
fn android_package_to_path(pkg: &str) -> String {
    pkg.replace('.', "/")
}

/// Append `org.gradle.java.home=<path>` to `Android/gradle.properties` when
/// we can detect a JDK 21 install via `/usr/libexec/java_home -v 21`.
///
/// The chunk-3c MANIFEST notes that `gradle.properties` deliberately omits
/// the line because it's per-machine; the scaffold writes it at init time
/// so the project is hermetic. On non-macOS hosts (where
/// `/usr/libexec/java_home` doesn't exist) the function is a no-op -- the
/// developer's `JAVA_HOME` is then the fallback, and the prereq check has
/// already verified `java --version` >= 21.
fn write_java_home(android_root: &Path) -> Result<(), VectisError> {
    let Some(java_home) = detect_java_home_21() else {
        return Ok(());
    };
    let path = android_root.join("gradle.properties");
    let mut contents = fs::read_to_string(&path)?;
    if !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str(&format!("org.gradle.java.home={java_home}\n"));
    fs::write(&path, contents)?;
    Ok(())
}

/// Detect a JDK 21 install path via macOS's `/usr/libexec/java_home`.
/// Returns `None` on non-macOS hosts (no `java_home` binary) or when no
/// JDK 21 is registered.
fn detect_java_home_21() -> Option<String> {
    let output = Command::new("/usr/libexec/java_home")
        .args(["-v", "21"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if raw.is_empty() { None } else { Some(raw) }
}

/// Write `Android/local.properties` with `sdk.dir=$ANDROID_HOME`.
///
/// `ANDROID_HOME` is a prereq for the `android` assembly (see
/// `prerequisites::TOOLS`), so we expect it to be set. If it isn't, return
/// `Internal` rather than silently producing a project that fails at the
/// first `gradle` invocation with a confusing error.
fn write_local_properties(android_root: &Path) -> Result<(), VectisError> {
    let android_home = std::env::var("ANDROID_HOME").map_err(|_| VectisError::Internal {
        message: "ANDROID_HOME is unset after prereq check passed; this should be unreachable"
            .into(),
    })?;
    let path = android_root.join("local.properties");
    let contents = format!("sdk.dir={android_home}\n");
    fs::write(&path, contents)?;
    Ok(())
}

/// Detect or read the NDK version to use.
///
/// Resolution: `versions.android.ndk` (when the user pinned one) wins;
/// otherwise we read `$ANDROID_HOME/ndk/<version>/` and pick the highest
/// version sorted lexically. Returns `Internal` if neither source yields a
/// version -- the prereq check guarantees at least one NDK directory
/// exists, so this should be unreachable in practice.
fn resolve_ndk_version(versions: &Versions) -> Result<String, VectisError> {
    if let Some(pinned) = versions.android.ndk.as_ref() {
        return Ok(pinned.clone());
    }
    let android_home = std::env::var("ANDROID_HOME").map_err(|_| VectisError::Internal {
        message: "ANDROID_HOME is unset after prereq check passed; this should be unreachable"
            .into(),
    })?;
    let ndk_root = PathBuf::from(android_home).join("ndk");
    let mut versions_found: Vec<String> = fs::read_dir(&ndk_root)
        .map_err(VectisError::from)?
        .filter_map(|entry| entry.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();
    if versions_found.is_empty() {
        return Err(VectisError::Internal {
            message: format!(
                "no NDK installs found under {} (prereq check should have caught this)",
                ndk_root.display()
            ),
        });
    }
    versions_found.sort_by(|a, b| {
        let parse = |s: &str| -> Vec<u64> {
            s.split('.').map(|c| c.parse::<u64>().unwrap_or(0)).collect()
        };
        parse(a).cmp(&parse(b))
    });
    Ok(versions_found.pop().expect("non-empty by check above"))
}

/// Bootstrap the Gradle wrapper, then run codegen + APK assembly.
///
/// Runs the chunk-3c pipeline:
///
/// 1. `gradle wrapper --gradle-version <pin>` -- bootstrap the wrapper in
///    a scratch directory containing only an empty `settings.gradle.kts`,
///    then copy the wrapper artefacts (`gradlew`, `gradlew.bat`,
///    `gradle/wrapper/{gradle-wrapper.jar,gradle-wrapper.properties}`)
///    into `Android/`. We don't run `gradle wrapper` directly inside
///    `Android/` because Gradle evaluates the project's settings + build
///    files first, which loads the `rust-android` plugin --
///    `rust-android-gradle = 0.9.6` calls `setFileMode(Integer)`, an API
///    Gradle 9.x removed (chunk-3c `MANIFEST.md § Verification
///    deviations`), so the bootstrap fails on any developer with system
///    Gradle 9.x. The empty-scratch-dir trick sidesteps the plugin load
///    entirely and works against both Gradle 8.x and 9.x system
///    installs.
/// 2. `make build` (codegen, from `Android/`) -- the Makefile delegates to
///    `cargo run --bin codegen` from the workspace root.
/// 3. `./gradlew :app:assembleDebug` (from `Android/`) -- runs against
///    the bootstrapped wrapper (Gradle 8.13), so `rust-android-gradle`'s
///    Gradle-9-incompatible API calls are not exercised.
///
/// Before running `:app:assembleDebug` we substitute the resolved NDK
/// version into `Android/shared/build.gradle.kts`. We can't do this at
/// scaffold time because the substitution lives outside the placeholder
/// table (`Versions::android.ndk` is `Option<String>` so the user can pin
/// it; if absent we detect from disk -- both paths land here).
fn run_pipeline(
    project_dir: &Path,
    android_root: &Path,
    versions: &Versions,
    _caps: &[Capability],
) -> Result<Vec<BuildStep>, VectisError> {
    // Resolve and inject the NDK version into the rendered shared
    // build.gradle.kts. Done lazily here (vs. during placeholder
    // substitution) so the file-system NDK lookup happens after prereqs
    // are checked and is bypassable in unit tests via `run_build=false`.
    let ndk_version = resolve_ndk_version(versions)?;
    let shared_build = android_root.join("shared/build.gradle.kts");
    let body = fs::read_to_string(&shared_build)?;
    let body = body.replace("__ANDROID_NDK_VERSION__", &ndk_version);
    fs::write(&shared_build, body)?;

    let _ = project_dir;

    let mut results = Vec::with_capacity(3);

    let gradle_pin = versions.android.gradle.clone();
    let wrapper_step = bootstrap_wrapper(android_root, &gradle_pin)?;
    let wrapper_passed = wrapper_step.passed;
    results.push(wrapper_step);
    if !wrapper_passed {
        return Err(VectisError::Verify {
            message: "Android build step `gradle wrapper` failed".to_string(),
        });
    }

    // The remaining two steps (`make build`, `./gradlew
    // :app:assembleDebug`) are identical to the verify-time Android
    // pipeline -- delegate so the step list cannot drift.
    let verify_steps = verify::android::run_pipeline(android_root)?;
    let failing = verify_steps.iter().find(|s| !s.passed).map(|s| s.name);
    results.extend(verify_steps);
    if let Some(name) = failing {
        return Err(VectisError::Verify {
            message: format!("Android build step `{name}` failed"),
        });
    }

    Ok(results)
}

/// Bootstrap the Gradle wrapper into `android_root` by running
/// `gradle wrapper` from a scratch directory.
///
/// See `run_pipeline` for the rationale. On success the four wrapper
/// artefacts (`gradlew`, `gradlew.bat`, `gradle/wrapper/gradle-wrapper.jar`,
/// `gradle/wrapper/gradle-wrapper.properties`) are present under
/// `android_root`. The scratch directory is best-effort cleaned up; a
/// leftover scratch dir is harmless.
fn bootstrap_wrapper(android_root: &Path, gradle_pin: &str) -> Result<BuildStep, VectisError> {
    let scratch = std::env::temp_dir().join(format!(
        "vectis-gradle-wrapper-bootstrap-{}",
        std::process::id()
    ));
    // Best-effort cleanup of any prior run; ignore errors -- they'll
    // surface via the `create_dir_all` below.
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch)?;
    fs::write(scratch.join("settings.gradle.kts"), "")?;

    let step = run_step(
        "gradle wrapper",
        Command::new("gradle")
            .args(["wrapper", "--gradle-version", gradle_pin])
            .current_dir(&scratch),
    )?;
    if !step.passed {
        let _ = fs::remove_dir_all(&scratch);
        return Ok(step);
    }

    // Move the four wrapper artefacts into `android_root`. We `copy` then
    // `remove_dir_all`-cleanup rather than `rename` because `/tmp` may be
    // on a different filesystem than the project (rename across mount
    // points fails with EXDEV on macOS).
    fs::create_dir_all(android_root.join("gradle/wrapper"))?;
    for (src_rel, dst_rel) in [
        ("gradlew", "gradlew"),
        ("gradlew.bat", "gradlew.bat"),
        (
            "gradle/wrapper/gradle-wrapper.jar",
            "gradle/wrapper/gradle-wrapper.jar",
        ),
        (
            "gradle/wrapper/gradle-wrapper.properties",
            "gradle/wrapper/gradle-wrapper.properties",
        ),
    ] {
        let src = scratch.join(src_rel);
        let dst = android_root.join(dst_rel);
        fs::copy(&src, &dst).map_err(VectisError::from)?;
    }
    // Preserve the executable bit on `gradlew` (fs::copy preserves mode
    // on macOS but not portably; set it explicitly).
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let gradlew = android_root.join("gradlew");
        let mut perms = fs::metadata(&gradlew)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&gradlew, perms)?;
    }

    let _ = fs::remove_dir_all(&scratch);
    Ok(step)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::{Mutex, MutexGuard, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Tests in this module mutate `ANDROID_HOME`. cargo runs unit tests
    /// in parallel by default; serialize through this mutex so two tests
    /// don't observe each other's `ANDROID_HOME` mid-scaffold.
    fn android_home_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn scratch_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        std::env::temp_dir().join(format!(
            "vectis-init-android-{label}-{}-{nanos}-{n}",
            std::process::id(),
        ))
    }

    fn sample_versions() -> Versions {
        Versions::embedded().expect("embedded defaults must parse")
    }

    fn sample_params() -> Params {
        let v = sample_versions();
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
            // Tests don't exercise the build pipeline, so the NDK
            // placeholder is left intact in `shared/build.gradle.kts`
            // until `run_pipeline` substitutes it. Use a recognisable
            // value so a regression that writes the placeholder into a
            // file *outside* the run-time substitution surfaces.
            android_ndk_version: "__ANDROID_NDK_VERSION__".into(),
        }
    }

    /// Set `ANDROID_HOME` for the duration of the test so
    /// `write_local_properties` can succeed without depending on the
    /// developer's environment. Serialised through `android_home_lock()`
    /// so two parallel tests cannot observe each other's value.
    fn with_android_home<F: FnOnce(&Path)>(dir: &Path, f: F) {
        let _guard = android_home_lock();
        let prev = std::env::var_os("ANDROID_HOME");
        unsafe {
            std::env::set_var("ANDROID_HOME", dir);
        }
        f(dir);
        unsafe {
            match prev {
                Some(v) => std::env::set_var("ANDROID_HOME", v),
                None => std::env::remove_var("ANDROID_HOME"),
            }
        }
    }

    #[test]
    fn android_package_to_path_translates_dots() {
        assert_eq!(
            android_package_to_path("com.vectis.counter"),
            "com/vectis/counter"
        );
        assert_eq!(android_package_to_path("com.example"), "com/example");
        assert_eq!(android_package_to_path("single"), "single");
    }

    #[test]
    fn scaffold_writes_every_android_file_for_render_only_minus_network_config() {
        let dir = scratch_dir("render-only");
        fs::create_dir_all(&dir).unwrap();
        let fake_sdk = scratch_dir("fake-sdk");
        fs::create_dir_all(&fake_sdk).unwrap();
        with_android_home(&fake_sdk, |_| {
            let result = scaffold(
                &dir,
                "com.vectis.counter",
                &[],
                &sample_params(),
                &sample_versions(),
                false,
            )
            .expect("scaffold must succeed");
            // 19 templates - 1 cap-conditional (network_security_config) = 18.
            assert_eq!(result.files.len(), android::TEMPLATES.len() - 1);
            assert!(
                !dir.join("Android/app/src/main/res/xml/network_security_config.xml")
                    .exists(),
                "network_security_config.xml must be skipped for render-only builds"
            );
            assert!(dir.join("Android/Makefile").is_file());
            assert!(dir.join("Android/build.gradle.kts").is_file());
            assert!(dir.join("Android/settings.gradle.kts").is_file());
            assert!(dir.join("Android/gradle.properties").is_file());
            assert!(dir.join("Android/gradle/libs.versions.toml").is_file());
            assert!(dir.join("Android/app/build.gradle.kts").is_file());
            assert!(dir.join("Android/shared/build.gradle.kts").is_file());
            assert!(
                dir.join("Android/app/src/main/AndroidManifest.xml")
                    .is_file()
            );
            assert!(
                dir.join(
                    "Android/app/src/main/java/com/vectis/counter/CounterApplication.kt"
                )
                .is_file()
            );
            assert!(
                dir.join("Android/app/src/main/java/com/vectis/counter/MainActivity.kt")
                    .is_file()
            );
            assert!(
                dir.join("Android/app/src/main/java/com/vectis/counter/core/Core.kt")
                    .is_file()
            );
            assert!(
                dir.join(
                    "Android/app/src/main/java/com/vectis/counter/ui/screens/HomeScreen.kt"
                )
                .is_file()
            );
            // local.properties must always be written.
            assert!(dir.join("Android/local.properties").is_file());
        });
    }

    #[test]
    fn scaffold_writes_network_security_config_when_http_enabled() {
        let dir = scratch_dir("http");
        fs::create_dir_all(&dir).unwrap();
        let fake_sdk = scratch_dir("fake-sdk-http");
        fs::create_dir_all(&fake_sdk).unwrap();
        with_android_home(&fake_sdk, |_| {
            let result = scaffold(
                &dir,
                "com.vectis.counter",
                &[Capability::Http],
                &sample_params(),
                &sample_versions(),
                false,
            )
            .expect("scaffold must succeed");
            assert_eq!(result.files.len(), android::TEMPLATES.len());
            assert!(
                dir.join("Android/app/src/main/res/xml/network_security_config.xml")
                    .is_file()
            );
            // HTTP cap markers should be kept inside libs.versions.toml.
            let libs = fs::read_to_string(dir.join("Android/gradle/libs.versions.toml")).unwrap();
            assert!(libs.contains("ktor = \"3.4.0\""));
            assert!(libs.contains("koinBom = \"4.1.1\""));
            assert!(!libs.contains("<<<CAP:"));
            // app-build.gradle.kts must keep the koin/ktor implementation
            // entries when http is on.
            let app_build = fs::read_to_string(dir.join("Android/app/build.gradle.kts")).unwrap();
            assert!(app_build.contains("koin.bom"));
            assert!(app_build.contains("ktor.client.core"));
            assert!(!app_build.contains("<<<CAP:"));
            // AndroidManifest.xml must reference the network_security_config.
            let manifest =
                fs::read_to_string(dir.join("Android/app/src/main/AndroidManifest.xml")).unwrap();
            assert!(manifest.contains("@xml/network_security_config"));
            assert!(!manifest.contains("<<<CAP:"));
        });
    }

    #[test]
    fn scaffold_substitutes_app_name_and_package_in_paths_and_contents() {
        let dir = scratch_dir("substitute");
        fs::create_dir_all(&dir).unwrap();
        let fake_sdk = scratch_dir("fake-sdk-sub");
        fs::create_dir_all(&fake_sdk).unwrap();
        with_android_home(&fake_sdk, |_| {
            scaffold(
                &dir,
                "com.example.noteeditor",
                &[],
                &Params {
                    app_name: "NoteEditor".into(),
                    app_struct: "NoteEditor".into(),
                    app_name_lower: "noteeditor".into(),
                    android_package: "com.example.noteeditor".into(),
                    ..sample_params()
                },
                &sample_versions(),
                false,
            )
            .unwrap();

            // Path-segment substitution.
            assert!(
                dir.join(
                    "Android/app/src/main/java/com/example/noteeditor/NoteEditorApplication.kt"
                )
                .is_file()
            );
            assert!(
                dir.join("Android/app/src/main/java/com/example/noteeditor/MainActivity.kt")
                    .is_file()
            );
            // File-content substitution.
            let app_kt = fs::read_to_string(
                dir.join(
                    "Android/app/src/main/java/com/example/noteeditor/NoteEditorApplication.kt",
                ),
            )
            .unwrap();
            assert!(!app_kt.contains("__APP_NAME__"));
            assert!(!app_kt.contains("__ANDROID_PACKAGE__"));
            assert!(app_kt.contains("package com.example.noteeditor"));
            assert!(app_kt.contains("class NoteEditorApplication"));
            // Settings.gradle.kts should carry the app name.
            let settings = fs::read_to_string(dir.join("Android/settings.gradle.kts")).unwrap();
            assert!(settings.contains("rootProject.name = \"NoteEditor\""));
        });
    }

    #[test]
    fn scaffold_strips_cap_blocks_for_render_only() {
        let dir = scratch_dir("strip-caps");
        fs::create_dir_all(&dir).unwrap();
        let fake_sdk = scratch_dir("fake-sdk-strip");
        fs::create_dir_all(&fake_sdk).unwrap();
        with_android_home(&fake_sdk, |_| {
            scaffold(
                &dir,
                "com.vectis.counter",
                &[],
                &sample_params(),
                &sample_versions(),
                false,
            )
            .unwrap();
            for entry in android::TEMPLATES {
                if !entry.include_when.should_include(&[]) {
                    continue;
                }
                let path = dir.join(substitute_path_with(
                    entry.target,
                    &sample_params(),
                    Some("com/vectis/counter"),
                ));
                let body = fs::read_to_string(&path).unwrap();
                assert!(
                    !body.contains("<<<CAP:"),
                    "leftover open marker in {}",
                    entry.target
                );
            }
            // Render-only: Core.kt's HTTP/KV/etc. arms should not appear.
            let core_kt =
                fs::read_to_string(dir.join("Android/app/src/main/java/com/vectis/counter/core/Core.kt"))
                    .unwrap();
            assert!(!core_kt.contains("Effect.Http"));
            assert!(!core_kt.contains("Effect.KeyValue"));
            assert!(!core_kt.contains("Effect.Time"));
            assert!(!core_kt.contains("Effect.Platform"));
            // libs.versions.toml: ktor/koin lines should be gone.
            let libs = fs::read_to_string(dir.join("Android/gradle/libs.versions.toml")).unwrap();
            assert!(!libs.contains("ktor"));
            assert!(!libs.contains("koin"));
        });
    }

    #[test]
    fn scaffold_writes_local_properties_with_android_home() {
        let dir = scratch_dir("local-props");
        fs::create_dir_all(&dir).unwrap();
        let fake_sdk = scratch_dir("fake-sdk-local");
        fs::create_dir_all(&fake_sdk).unwrap();
        with_android_home(&fake_sdk, |sdk_path| {
            scaffold(
                &dir,
                "com.vectis.counter",
                &[],
                &sample_params(),
                &sample_versions(),
                false,
            )
            .unwrap();
            let local = fs::read_to_string(dir.join("Android/local.properties")).unwrap();
            assert!(
                local.contains(&format!("sdk.dir={}", sdk_path.display())),
                "expected sdk.dir line in local.properties, got: {local}"
            );
        });
    }

    #[test]
    fn scaffold_refuses_to_overwrite_existing_android_dir() {
        let dir = scratch_dir("no-overwrite");
        fs::create_dir_all(dir.join("Android")).unwrap();
        let fake_sdk = scratch_dir("fake-sdk-no-overwrite");
        fs::create_dir_all(&fake_sdk).unwrap();
        with_android_home(&fake_sdk, |_| {
            let err = scaffold(
                &dir,
                "com.vectis.counter",
                &[],
                &sample_params(),
                &sample_versions(),
                false,
            )
            .expect_err("must refuse to overwrite Android/");
            match err {
                VectisError::InvalidProject { message } => {
                    assert!(message.contains("refusing to overwrite existing Android shell"));
                }
                other => panic!("unexpected: {other:?}"),
            }
            assert!(!dir.join("Android/Makefile").exists());
        });
    }
}

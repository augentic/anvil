//! `vectis init` -- scaffold a new Crux project.
//!
//! Chunk 5 landed the render-only core scaffold. Chunk 6 wires the
//! `--caps` flag through the engine so any combination of `http`, `kv`,
//! `time`, `platform`, `sse` is honoured. Chunk 7 adds the iOS shell:
//! `--shells ios` writes the chunk-3b iOS templates under
//! `iOS/<AppName>/...` and (when prerequisites pass) drives `make typegen
//! && make package && make xcode` from `iOS/`. Chunk 8 adds the Android
//! shell: `--shells android` writes the chunk-3c Android templates under
//! `Android/...`, bootstraps the Gradle wrapper, writes
//! `local.properties` from `$ANDROID_HOME`, and (when prerequisites pass)
//! drives `make build` + `./gradlew :app:assembleDebug` from `Android/`.

pub mod android;
pub mod core;
pub mod ios;

use std::path::PathBuf;

use crate::{
    CommandOutcome, InitArgs,
    error::VectisError,
    prerequisites::{self, AssemblyKind},
    templates::{Capability, Params},
    versions::Versions,
};

pub fn run(args: &InitArgs) -> Result<CommandOutcome, VectisError> {
    let mut assemblies = vec![AssemblyKind::Core];
    let shells = parse_shells(args.shells.as_deref())?;
    for shell in &shells {
        assemblies.push(*shell);
    }

    prerequisites::check(&assemblies)?;

    // Resolve version pins up-front so a bad `--version-file` is reported
    // before we touch the filesystem (chunk 4 wired this in for the smoke
    // test; chunk 5 starts actually consuming the resolved struct).
    let project_dir = resolve_project_dir(args.dir.as_deref())?;
    let versions = Versions::resolve(&project_dir, args.version_file.as_deref())?;

    let caps = parse_caps(args.caps.as_deref())?;

    let android_package = args
        .android_package
        .clone()
        .unwrap_or_else(|| core::default_android_package(&args.app_name));

    let core_result = core::scaffold(
        &project_dir,
        &args.app_name,
        &android_package,
        &versions,
        &caps,
    )?;

    let mut assemblies_json = serde_json::Map::new();
    assemblies_json.insert(
        "core".to_string(),
        serde_json::json!({
            "status": "created",
            "files": core_result.files,
        }),
    );

    let mut shells_emitted: Vec<&'static str> = Vec::new();

    // Chunk 7 reuses the same Params placeholder map the core scaffold
    // built. We rebuild it here rather than threading it out of
    // `core::scaffold` because it is tiny and rebuilding decouples the
    // public signatures of the per-shell scaffolders. Chunk 8 will reuse
    // the same builder.
    let params = build_params(&args.app_name, &android_package, &versions);

    for shell in &shells {
        match shell {
            AssemblyKind::Ios => {
                let ios_result =
                    ios::scaffold(&project_dir, &args.app_name, &caps, &params, true)?;
                assemblies_json.insert(
                    "ios".to_string(),
                    serde_json::json!({
                        "status": "created",
                        "files": ios_result.files,
                        "build_steps": ios_result.build_steps,
                    }),
                );
                shells_emitted.push("ios");
            }
            AssemblyKind::Android => {
                let android_result = android::scaffold(
                    &project_dir,
                    &android_package,
                    &caps,
                    &params,
                    &versions,
                    true,
                )?;
                assemblies_json.insert(
                    "android".to_string(),
                    serde_json::json!({
                        "status": "created",
                        "files": android_result.files,
                        "build_steps": android_result.build_steps,
                    }),
                );
                shells_emitted.push("android");
            }
            AssemblyKind::Core => {
                // Core is already in `assemblies` for the prereq scope;
                // it is never listed in `--shells`.
                unreachable!("parse_shells filters out core");
            }
        }
    }

    let value = serde_json::json!({
        "app_name": args.app_name,
        "app_struct": args.app_name,
        "project_dir": project_dir.display().to_string(),
        "assemblies": assemblies_json,
        "capabilities": caps.iter().map(|c| c.marker_tag()).collect::<Vec<_>>(),
        "shells": shells_emitted,
    });

    Ok(CommandOutcome::Success(value))
}

/// Build the placeholder map shared by every per-assembly scaffold.
///
/// Single source of truth for placeholder construction -- used by
/// `init::core::scaffold`, the shell scaffolders, and `add_shell::run`.
pub(crate) fn build_params(app_name: &str, android_package: &str, versions: &Versions) -> Params {
    Params {
        app_name: app_name.to_string(),
        app_struct: app_name.to_string(),
        app_name_lower: app_name.to_lowercase(),
        android_package: android_package.to_string(),
        crux_core_version: versions.crux.crux_core.clone(),
        crux_http_version: versions.crux.crux_http.clone(),
        crux_kv_version: versions.crux.crux_kv.clone(),
        crux_time_version: versions.crux.crux_time.clone(),
        crux_platform_version: versions.crux.crux_platform.clone(),
        facet_version: versions.crux.facet.clone(),
        serde_version: versions.crux.serde.clone(),
        uniffi_version: versions.crux.uniffi.clone(),
        agp_version: versions.android.agp.clone(),
        kotlin_version: versions.android.kotlin.clone(),
        compose_bom_version: versions.android.compose_bom.clone(),
        ktor_version: versions.android.ktor.clone(),
        koin_version: versions.android.koin.clone(),
        // `__ANDROID_NDK_VERSION__` is *not* substituted at scaffold time --
        // chunk 8's `init::android::run_pipeline` resolves it (from
        // `versions.android.ndk` or by scanning `$ANDROID_HOME/ndk/`) and
        // patches `Android/shared/build.gradle.kts` just before the build
        // pipeline runs. Leaving the placeholder visible here lets the
        // scaffolded file round-trip through unit tests without depending
        // on a per-machine NDK install.
        android_ndk_version: "__ANDROID_NDK_VERSION__".to_string(),
    }
}

/// Parse the `--caps` flag into the canonical `Capability` set.
///
/// Accepts a comma-separated list. Empty entries (including `--caps ""`)
/// are tolerated so build orchestration that always passes the flag does
/// not break. Unknown tags produce an `InvalidProject` error pointing at
/// the offending value and the canonical accepted set.
///
/// Duplicate entries are deduplicated in input order so the rendered
/// output is stable regardless of how the user spells the list.
fn parse_caps(raw: Option<&str>) -> Result<Vec<Capability>, VectisError> {
    let mut out: Vec<Capability> = Vec::new();
    let Some(raw) = raw else { return Ok(out) };
    for tag in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let cap = Capability::from_tag(tag).ok_or_else(|| VectisError::InvalidProject {
            message: format!(
                "unknown capability: {tag:?} (expected one of: http, kv, time, platform, sse)"
            ),
        })?;
        if !out.contains(&cap) {
            out.push(cap);
        }
    }
    Ok(out)
}

fn parse_shells(raw: Option<&str>) -> Result<Vec<AssemblyKind>, VectisError> {
    let mut out = Vec::new();
    let Some(raw) = raw else { return Ok(out) };
    for shell in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        match shell {
            "ios" => out.push(AssemblyKind::Ios),
            "android" => out.push(AssemblyKind::Android),
            other => {
                return Err(VectisError::InvalidProject {
                    message: format!(
                        "unknown shell platform: {other:?} (expected one of: ios, android)"
                    ),
                });
            }
        }
    }
    Ok(out)
}

fn resolve_project_dir(dir: Option<&std::path::Path>) -> Result<PathBuf, VectisError> {
    match dir {
        Some(p) => Ok(p.to_path_buf()),
        None => std::env::current_dir().map_err(VectisError::from),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_caps_none_yields_empty_render_only_set() {
        assert!(parse_caps(None).unwrap().is_empty());
    }

    #[test]
    fn parse_caps_empty_string_is_render_only() {
        // `--caps ""` and `--caps " , "` must both behave like "no caps"
        // so callers (CI, scripts) can pass the flag unconditionally.
        assert!(parse_caps(Some("")).unwrap().is_empty());
        assert!(parse_caps(Some(" , ,")).unwrap().is_empty());
    }

    #[test]
    fn parse_caps_accepts_full_matrix_in_order() {
        let caps = parse_caps(Some("http,kv,time,platform,sse")).unwrap();
        assert_eq!(
            caps,
            vec![
                Capability::Http,
                Capability::Kv,
                Capability::Time,
                Capability::Platform,
                Capability::Sse,
            ]
        );
    }

    #[test]
    fn parse_caps_trims_whitespace_around_each_token() {
        let caps = parse_caps(Some("  http , kv ")).unwrap();
        assert_eq!(caps, vec![Capability::Http, Capability::Kv]);
    }

    #[test]
    fn parse_caps_dedupes_in_input_order() {
        let caps = parse_caps(Some("kv,http,kv,http,time")).unwrap();
        assert_eq!(
            caps,
            vec![Capability::Kv, Capability::Http, Capability::Time]
        );
    }

    #[test]
    fn parse_caps_rejects_unknown_token() {
        let err = parse_caps(Some("http,bogus")).expect_err("unknown cap must error");
        match err {
            VectisError::InvalidProject { message } => {
                assert!(message.contains("\"bogus\""), "{message}");
                assert!(message.contains("http"), "{message}");
                assert!(message.contains("sse"), "{message}");
            }
            other => panic!("unexpected: {other:?}"),
        }
    }
}

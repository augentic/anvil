//! Version pin resolution -- embedded defaults, user config, project file,
//! and an explicit `--version-file` override.
//!
//! See RFC-6 § Version Management. Resolution order (top to bottom, first
//! hit wins):
//!
//! 1. `--version-file <path>` (explicit override; must exist)
//! 2. `<project>/versions.toml`
//! 3. `~/.config/vectis/versions.toml`
//! 4. embedded defaults compiled into the binary
//!
//! Every layer is a complete `Versions` document -- no partial / merge
//! semantics in chunk 4. Adding merge later is feasible without breaking
//! callers; not doing it now keeps the parsing surface tight and
//! deterministic.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::error::VectisError;

/// The raw text of the embedded defaults, compiled in at build time.
///
/// The path is relative to this source file: `src/versions.rs` -> the
/// `embedded/versions.toml` sibling of `src/`.
const EMBEDDED_DEFAULTS: &str = include_str!("../embedded/versions.toml");

/// Top-level pinned version document.
///
/// Substruct field names match the placeholders the templates substitute
/// (chunk 3a/3b/3c MANIFESTs); chunks 5/6/7/8/11 read directly off these
/// fields.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Versions {
    pub crux: Crux,
    pub android: Android,
    #[serde(default)]
    #[allow(dead_code)] // populated for forward compat; no consumers in chunks 4-11
    pub ios: Ios,
    pub tooling: Tooling,
}

/// Crux + transitive Rust pins. The hard-pinned set (`facet`,
/// `facet_generate`, `serde*`, `uniffi`, `cargo_swift`) moves in lockstep
/// with `crux_core`; chunk 11's `update-versions` is responsible for
/// proving coherence whenever any of them bumps.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Crux {
    pub crux_core: String,
    pub crux_http: String,
    pub crux_kv: String,
    pub crux_time: String,
    pub crux_platform: String,
    pub facet: String,
    pub facet_generate: String,
    pub serde: String,
    pub serde_json: String,
    pub uniffi: String,
    pub cargo_swift: String,
}

/// Android toolchain pins. `ndk` is `Option<String>`: when absent, chunk 8
/// detects the installed version from `$ANDROID_HOME/ndk/<version>/` at
/// scaffold time rather than failing on a pin the developer does not have
/// installed.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Android {
    pub compose_bom: String,
    pub koin: String,
    pub ktor: String,
    pub kotlin: String,
    pub agp: String,
    pub gradle: String,
    #[serde(default)]
    pub ndk: Option<String>,
}

/// iOS pins. Empty today (every dependency is a generated, internal SPM
/// package); kept as a substruct so external SPM pins can land later
/// without changing the resolver shape.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub struct Ios {}

/// Tooling pins (CLI binaries the developer installs separately). Mostly
/// informational today; chunk 11 will use them when querying registries.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Tooling {
    pub cargo_deny: String,
    pub cargo_vet: String,
    pub xcodegen: String,
}

impl Versions {
    /// Resolve the active version pins for a project.
    ///
    /// `project_dir` is the directory the CLI is operating on (the
    /// `--dir` argument, or the current working directory). `override_path`
    /// is the value of `--version-file` if the user passed one; when set,
    /// the file MUST exist and parse, otherwise a structured
    /// `InvalidProject` error is returned.
    pub fn resolve(
        project_dir: &Path,
        override_path: Option<&Path>,
    ) -> Result<Versions, VectisError> {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        Self::resolve_with(project_dir, override_path, home.as_deref())
    }

    /// Inner resolver with an explicit `home_dir` for testability. The
    /// public `resolve` delegates here after reading `$HOME`.
    fn resolve_with(
        project_dir: &Path,
        override_path: Option<&Path>,
        home_dir: Option<&Path>,
    ) -> Result<Versions, VectisError> {
        if let Some(path) = override_path {
            return load_required(path);
        }
        let project_path = project_dir.join("versions.toml");
        if project_path.is_file() {
            return load_required(&project_path);
        }
        if let Some(home) = home_dir {
            let user_path = home.join(".config").join("vectis").join("versions.toml");
            if user_path.is_file() {
                return load_required(&user_path);
            }
        }
        load_embedded()
    }

    /// Parse the embedded defaults. Public so chunks that legitimately want
    /// the baseline (e.g. `update-versions --dry-run` showing "current"
    /// when no file exists yet) can call it directly.
    pub fn embedded() -> Result<Versions, VectisError> {
        load_embedded()
    }
}

fn load_required(path: &Path) -> Result<Versions, VectisError> {
    if !path.exists() {
        return Err(VectisError::InvalidProject {
            message: format!("version file not found: {}", path.display()),
        });
    }
    let contents = std::fs::read_to_string(path)?;
    parse(&contents).map_err(|err| VectisError::InvalidProject {
        message: format!("failed to parse {}: {err}", path.display()),
    })
}

fn load_embedded() -> Result<Versions, VectisError> {
    parse(EMBEDDED_DEFAULTS).map_err(|err| VectisError::Internal {
        message: format!("embedded versions.toml is malformed: {err}"),
    })
}

fn parse(contents: &str) -> Result<Versions, toml::de::Error> {
    toml::from_str(contents)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Returns a unique scratch directory inside `std::env::temp_dir()`.
    /// Process-pid + monotonic counter + nanosecond-grained time keeps
    /// every test isolated even when run with `--test-threads=N>1`.
    fn scratch_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "vectis-versions-{label}-{}-{nanos}-{n}",
            std::process::id(),
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn minimal_versions_toml(crux_core: &str) -> String {
        format!(
            r#"
[crux]
crux_core = "{crux_core}"
crux_http = "0.0.0"
crux_kv = "0.0.0"
crux_time = "0.0.0"
crux_platform = "0.0.0"
facet = "=0.0"
facet_generate = "=0.0"
serde = "0.0"
serde_json = "0.0"
uniffi = "=0.0.0"
cargo_swift = "0.0"

[android]
compose_bom = "0000.00.00"
koin = "0.0.0"
ktor = "0.0.0"
kotlin = "0.0.0"
agp = "0.0.0"
gradle = "0.0"

[tooling]
cargo_deny = "0.0.0"
cargo_vet = "0.0.0"
xcodegen = "0.0.0"
"#
        )
    }

    #[test]
    fn embedded_defaults_parse_and_match_initial_pins() {
        let v = Versions::embedded().expect("embedded defaults must parse");
        assert_eq!(v.crux.crux_core, "0.17.0");
        assert_eq!(v.crux.crux_http, "0.16.0");
        assert_eq!(v.crux.crux_kv, "0.11.0");
        assert_eq!(v.crux.crux_time, "0.15.0");
        assert_eq!(v.crux.crux_platform, "0.8.0");
        assert_eq!(v.crux.facet, "=0.31");
        assert_eq!(v.crux.facet_generate, "=0.15");
        assert_eq!(v.crux.uniffi, "=0.29.4");
        assert_eq!(v.crux.cargo_swift, "0.9");
        // Android values are the chunk-3c bumped pins, not the original
        // RFC block (which doesn't produce a buildable APK on Xcode 16
        // / Java 21).
        assert_eq!(v.android.agp, "8.13.2");
        assert_eq!(v.android.kotlin, "2.3.0");
        assert_eq!(v.android.compose_bom, "2026.01.01");
        assert_eq!(v.android.ktor, "3.4.0");
        assert_eq!(v.android.koin, "4.1.1");
        assert_eq!(v.android.gradle, "8.13");
        // `ndk` is intentionally absent; chunk 8 detects from disk.
        assert!(v.android.ndk.is_none());
        assert_eq!(v.tooling.cargo_deny, "0.19.4");
        assert_eq!(v.tooling.cargo_vet, "0.10.2");
        assert_eq!(v.tooling.xcodegen, "2.42.0");
    }

    #[test]
    fn embedded_layer_used_when_no_files_or_overrides() {
        let project = scratch_dir("embedded-only");
        let home = scratch_dir("embedded-only-home");
        let v =
            Versions::resolve_with(&project, None, Some(&home)).expect("embedded must resolve");
        let baseline = Versions::embedded().unwrap();
        assert_eq!(v, baseline);
    }

    #[test]
    fn override_layer_takes_precedence_over_everything() {
        let project = scratch_dir("override");
        let home = scratch_dir("override-home");
        // Plant a project-local versions.toml that should be ignored
        // because the override takes precedence.
        fs::write(
            project.join("versions.toml"),
            minimal_versions_toml("9.9.9"),
        )
        .unwrap();
        let override_path = scratch_dir("override-file").join("pins.toml");
        fs::write(&override_path, minimal_versions_toml("1.2.3")).unwrap();

        let v = Versions::resolve_with(&project, Some(&override_path), Some(&home)).unwrap();
        assert_eq!(v.crux.crux_core, "1.2.3");
    }

    #[test]
    fn project_layer_takes_precedence_over_user_and_embedded() {
        let project = scratch_dir("project");
        let home = scratch_dir("project-home");
        fs::write(
            project.join("versions.toml"),
            minimal_versions_toml("4.5.6"),
        )
        .unwrap();
        // Plant a user file that should be ignored.
        let user = home.join(".config").join("vectis");
        fs::create_dir_all(&user).unwrap();
        fs::write(user.join("versions.toml"), minimal_versions_toml("8.8.8")).unwrap();

        let v = Versions::resolve_with(&project, None, Some(&home)).unwrap();
        assert_eq!(v.crux.crux_core, "4.5.6");
    }

    #[test]
    fn user_layer_takes_precedence_over_embedded() {
        let project = scratch_dir("user");
        let home = scratch_dir("user-home");
        let user = home.join(".config").join("vectis");
        fs::create_dir_all(&user).unwrap();
        fs::write(user.join("versions.toml"), minimal_versions_toml("7.7.7")).unwrap();

        let v = Versions::resolve_with(&project, None, Some(&home)).unwrap();
        assert_eq!(v.crux.crux_core, "7.7.7");
    }

    #[test]
    fn missing_override_returns_invalid_project_error() {
        let project = scratch_dir("missing-override");
        let home = scratch_dir("missing-override-home");
        let bogus = PathBuf::from("/this/path/should/never/exist/versions.toml");

        let err = Versions::resolve_with(&project, Some(&bogus), Some(&home))
            .expect_err("nonexistent override must error");
        match err {
            VectisError::InvalidProject { message } => {
                assert!(
                    message.contains("version file not found"),
                    "unexpected message: {message}"
                );
                assert!(
                    message.contains("/this/path/should/never/exist/versions.toml"),
                    "message must include the missing path: {message}"
                );
            }
            other => panic!("expected InvalidProject, got: {other:?}"),
        }
    }

    #[test]
    fn malformed_override_returns_invalid_project_error_with_path() {
        let project = scratch_dir("malformed-override");
        let home = scratch_dir("malformed-override-home");
        let bad = scratch_dir("malformed-override-file").join("bad.toml");
        fs::write(&bad, "this = is = not = toml = at = all\n").unwrap();

        let err = Versions::resolve_with(&project, Some(&bad), Some(&home))
            .expect_err("malformed override must error");
        match err {
            VectisError::InvalidProject { message } => {
                assert!(
                    message.contains("failed to parse"),
                    "unexpected message: {message}"
                );
                assert!(
                    message.contains(bad.to_string_lossy().as_ref()),
                    "message must include the path: {message}"
                );
            }
            other => panic!("expected InvalidProject, got: {other:?}"),
        }
    }

    #[test]
    fn directory_passed_as_override_returns_invalid_project_error() {
        // `--version-file <some-existing-directory>` should fail clean,
        // not panic from `read_to_string`.
        let project = scratch_dir("dir-override");
        let home = scratch_dir("dir-override-home");
        let dir = scratch_dir("dir-override-target");
        let err = Versions::resolve_with(&project, Some(&dir), Some(&home))
            .expect_err("directory override must error");
        // Either Io (read_to_string on a directory) or InvalidProject
        // (toml parser) is acceptable; what matters is no panic and a
        // non-zero exit through the existing error machinery.
        match err {
            VectisError::Io(_) | VectisError::InvalidProject { .. } => {}
            other => panic!("expected Io or InvalidProject, got: {other:?}"),
        }
    }

    #[test]
    fn no_home_falls_through_to_embedded() {
        let project = scratch_dir("no-home");
        let v = Versions::resolve_with(&project, None, None).unwrap();
        assert_eq!(v, Versions::embedded().unwrap());
    }
}

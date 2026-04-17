//! `vectis update-versions` -- query registries, compute a coherent
//! bump, and either diff-print (`--dry-run`) or atomically write the
//! result to the target pins file.
//!
//! The pipeline, in order:
//!
//! 1. Resolve the write target (defaults to
//!    `$HOME/.config/vectis/versions.toml`). The `--version-file` flag
//!    on this subcommand is the *write target*, not a resolution
//!    override (as it is on `init` / `verify` / `add-shell`).
//! 2. Read the current pins (target file if it exists, otherwise
//!    embedded defaults).
//! 3. Scope the prerequisite check: core-only by default; core + iOS +
//!    Android when `--verify` is set (the verify matrix needs the full
//!    toolchain to build).
//! 4. Query registries for a new candidate `Versions`. Per-field
//!    failures are non-fatal: they keep the current pin and add an
//!    entry to the `errors` array in the output JSON so the user sees
//!    what the registry probe missed. This keeps the command usable
//!    on a flaky connection.
//! 5. Compute the diff (key, current, proposed).
//! 6. When `--verify` is set, scaffold each entry in
//!    [`matrix::CAP_MATRIX`] with the proposed pins and run
//!    `vectis verify` against each. The matrix passes only if every
//!    combo does.
//! 7. Write semantics:
//!    - `--dry-run`: never write.
//!    - no `--dry-run`, no `--verify`: always write.
//!    - no `--dry-run`, `--verify`: write only if the matrix passed.
//!
//! Writes are atomic: we render the proposed TOML to a sibling
//! `<target>.tmp` file, then rename it over the target. A crash
//! mid-write leaves the original file intact.

mod matrix;
mod query;

use std::fs;
use std::path::{Path, PathBuf};

use crate::{
    CommandOutcome, UpdateVersionsArgs,
    error::VectisError,
    prerequisites::{self, AssemblyKind},
    versions::{Android, Crux, Ios, Tooling, Versions},
};

/// Result of proposing one version field. When the registry probe
/// fails we keep the current value and surface the error; the diff
/// output shows "unchanged" for the field and the error array carries
/// the diagnostic.
struct ProposedField {
    value: String,
    /// Set only when the probe failed; the current value was kept.
    probe_error: Option<String>,
}

impl ProposedField {
    fn current(value: String) -> Self {
        Self {
            value,
            probe_error: None,
        }
    }
}

/// Try a registry query; on success take the resolved version, on
/// failure keep the current pin and record the probe error.
fn try_query<F>(current: &str, probe: F, context: &str) -> ProposedField
where
    F: FnOnce() -> Result<query::VersionHit, VectisError>,
{
    match probe() {
        Ok(hit) => ProposedField {
            value: hit.version,
            probe_error: None,
        },
        Err(err) => ProposedField {
            value: current.to_string(),
            probe_error: Some(format!("{context}: {err}")),
        },
    }
}

/// Try to derive a version from arbitrary logic (e.g. extracting a dep
/// requirement); same keep-on-failure semantics as [`try_query`].
fn try_derive<F>(current: &str, derive: F, context: &str) -> ProposedField
where
    F: FnOnce() -> Result<String, VectisError>,
{
    match derive() {
        Ok(value) => ProposedField {
            value,
            probe_error: None,
        },
        Err(err) => ProposedField {
            value: current.to_string(),
            probe_error: Some(format!("{context}: {err}")),
        },
    }
}

pub fn run(args: &UpdateVersionsArgs) -> Result<CommandOutcome, VectisError> {
    let assemblies: Vec<AssemblyKind> = if args.verify {
        vec![AssemblyKind::Core, AssemblyKind::Ios, AssemblyKind::Android]
    } else {
        vec![AssemblyKind::Core]
    };
    prerequisites::check(&assemblies)?;

    let target = resolve_write_target(args.version_file.as_deref())?;
    let current = read_current_pins(&target)?;

    let mut errors: Vec<String> = Vec::new();
    let proposed = compute_proposed(&current, &mut errors);

    let changes = diff_fields(&current, &proposed);
    let unchanged = unchanged_fields(&current, &proposed);

    // Run the verify matrix *before* writing so a failing matrix never
    // clobbers the user's existing pins.
    let verification = if args.verify {
        let scratch = verify_scratch_dir(std::process::id());
        let _ = fs::remove_dir_all(&scratch);
        let matrix_result = matrix::run(&proposed, &scratch)?;
        let _ = fs::remove_dir_all(&scratch);
        Some(matrix_result)
    } else {
        None
    };

    let matrix_passed = verification.as_ref().map(|m| m.passed).unwrap_or(true);

    let mut written = false;
    if !args.dry_run && matrix_passed {
        write_atomic(&target, &matrix::render_pins_toml(&proposed))?;
        written = true;
    }

    let overall_passed = matrix_passed;

    let mut json = serde_json::Map::new();
    json.insert(
        "version_file".to_string(),
        serde_json::Value::String(target.display().to_string()),
    );
    json.insert("dry_run".to_string(), serde_json::Value::Bool(args.dry_run));
    json.insert("verify".to_string(), serde_json::Value::Bool(args.verify));
    json.insert(
        "passed".to_string(),
        serde_json::Value::Bool(overall_passed),
    );
    json.insert("written".to_string(), serde_json::Value::Bool(written));
    json.insert(
        "changes".to_string(),
        serde_json::to_value(&changes).unwrap(),
    );
    json.insert(
        "unchanged".to_string(),
        serde_json::to_value(&unchanged).unwrap(),
    );
    if !errors.is_empty() {
        json.insert("errors".to_string(), serde_json::to_value(&errors).unwrap());
    }
    if let Some(ver) = verification {
        json.insert(
            "verification".to_string(),
            serde_json::to_value(&ver).unwrap(),
        );
    }

    Ok(CommandOutcome::Success(serde_json::Value::Object(json)))
}

/// Resolve the write target: an explicit `--version-file` if provided,
/// otherwise `$HOME/.config/vectis/versions.toml`. Fails with
/// `Internal` when `$HOME` is unset and no path was given -- the CLI
/// has no sane default in that case.
fn resolve_write_target(override_path: Option<&Path>) -> Result<PathBuf, VectisError> {
    if let Some(path) = override_path {
        return Ok(path.to_path_buf());
    }
    let home = std::env::var_os("HOME").ok_or_else(|| VectisError::Internal {
        message: "$HOME is unset; pass --version-file <path> explicitly".to_string(),
    })?;
    Ok(PathBuf::from(home)
        .join(".config")
        .join("vectis")
        .join("versions.toml"))
}

/// Read the pins currently recorded at the write target.
///
/// If the target exists, parse it. If it does not exist, fall back to
/// the embedded defaults -- this is the "first run" case where the
/// user has never materialised their pins file and
/// `update-versions --dry-run` is showing "here's what I would
/// create". A resolution error on an *existing* file is returned as
/// `InvalidProject` so the user can see which file is bad; this
/// mirrors the semantics of `Versions::resolve` on the other
/// subcommands.
fn read_current_pins(target: &Path) -> Result<Versions, VectisError> {
    if target.is_file() {
        let contents = fs::read_to_string(target)?;
        return toml::from_str(&contents).map_err(|err| VectisError::InvalidProject {
            message: format!("failed to parse {}: {err}", target.display()),
        });
    }
    Versions::embedded()
}

/// Query every registry surface and build a proposed `Versions`. Probe
/// failures are recorded in `errors` and cause the field to keep its
/// current value.
fn compute_proposed(current: &Versions, errors: &mut Vec<String>) -> Versions {
    // Crux block ----------------------------------------------------
    let crux_core =
        try_query(&current.crux.crux_core, || query::crates_io_latest_stable("crux_core"),
            "crates.io: crux_core");
    record_err(errors, &crux_core.probe_error);

    // For each capability crate, query latest stable. When crux_core
    // resolved successfully we could additionally verify the cap's
    // crux_core dep req; we skip that here because the --verify
    // matrix is the authoritative coherence gate and a parse-level
    // semver check is more complex than it's worth at MVP.
    let crux_http =
        try_query(&current.crux.crux_http, || query::crates_io_latest_stable("crux_http"),
            "crates.io: crux_http");
    record_err(errors, &crux_http.probe_error);

    let crux_kv = try_query(&current.crux.crux_kv, || query::crates_io_latest_stable("crux_kv"),
            "crates.io: crux_kv");
    record_err(errors, &crux_kv.probe_error);

    let crux_time =
        try_query(&current.crux.crux_time, || query::crates_io_latest_stable("crux_time"),
            "crates.io: crux_time");
    record_err(errors, &crux_time.probe_error);

    let crux_platform =
        try_query(&current.crux.crux_platform, || query::crates_io_latest_stable("crux_platform"),
            "crates.io: crux_platform");
    record_err(errors, &crux_platform.probe_error);

    // Hard-pinned Rust deps derived from crux_core's own Cargo.toml.
    // If crux_core failed to resolve we keep the current values
    // across the board (no point deriving a req out of a version we
    // don't have).
    let facet = if crux_core.probe_error.is_none() {
        try_derive(
            &current.crux.facet,
            || query::crates_io_normal_dep_req("crux_core", &crux_core.value, "facet"),
            "crates.io: crux_core deps (facet)",
        )
    } else {
        ProposedField::current(current.crux.facet.clone())
    };
    record_err(errors, &facet.probe_error);

    let facet_generate = if crux_core.probe_error.is_none() {
        try_derive(
            &current.crux.facet_generate,
            || query::crates_io_normal_dep_req("crux_core", &crux_core.value, "facet_generate"),
            "crates.io: crux_core deps (facet_generate)",
        )
    } else {
        ProposedField::current(current.crux.facet_generate.clone())
    };
    record_err(errors, &facet_generate.probe_error);

    let serde =
        try_query(&current.crux.serde, || query::crates_io_latest_stable("serde"),
            "crates.io: serde");
    record_err(errors, &serde.probe_error);

    let serde_json =
        try_query(&current.crux.serde_json, || query::crates_io_latest_stable("serde_json"),
            "crates.io: serde_json");
    record_err(errors, &serde_json.probe_error);

    // uniffi + cargo-swift move as a pair: cargo-swift's dep on
    // uniffi_bindgen drives which uniffi runtime pin we use.
    let cargo_swift =
        try_query(&current.crux.cargo_swift, || query::crates_io_latest_stable("cargo-swift"),
            "crates.io: cargo-swift");
    record_err(errors, &cargo_swift.probe_error);

    let uniffi = if cargo_swift.probe_error.is_none() {
        try_derive(
            &current.crux.uniffi,
            || {
                // cargo-swift's uniffi_bindgen dep is a `normal` dep
                // on a crate called `uniffi_bindgen`. The req string
                // (e.g. `=0.31.0`) is exactly the shape we want to
                // pin into versions.toml for the runtime `uniffi`
                // crate, because cargo-swift locks the bindgen and
                // runtime to the same version.
                query::crates_io_normal_dep_req(
                    "cargo-swift",
                    &cargo_swift.value,
                    "uniffi_bindgen",
                )
            },
            "crates.io: cargo-swift deps (uniffi_bindgen)",
        )
    } else {
        ProposedField::current(current.crux.uniffi.clone())
    };
    record_err(errors, &uniffi.probe_error);

    // Android block -------------------------------------------------
    // Compose BOM lives on Google Maven. kotlin + AGP are on Maven
    // Central (kotlin-stdlib) and Google Maven respectively. koin and
    // ktor publish on Maven Central via BOMs.
    let compose_bom = try_query(
        &current.android.compose_bom,
        || query::google_maven_latest_stable("androidx.compose", "compose-bom"),
        "google maven: compose-bom",
    );
    record_err(errors, &compose_bom.probe_error);

    let kotlin = try_query(
        &current.android.kotlin,
        || query::maven_central_latest_stable("org.jetbrains.kotlin", "kotlin-stdlib"),
        "maven central: kotlin-stdlib",
    );
    record_err(errors, &kotlin.probe_error);

    // AGP is published as a Gradle plugin marker artefact. The marker
    // artefact version tracks the plugin version directly.
    let agp = try_query(
        &current.android.agp,
        || {
            query::google_maven_latest_stable(
                "com.android.application",
                "com.android.application.gradle.plugin",
            )
        },
        "google maven: com.android.application",
    );
    record_err(errors, &agp.probe_error);

    let koin = try_query(
        &current.android.koin,
        || query::maven_central_latest_stable("io.insert-koin", "koin-bom"),
        "maven central: koin-bom",
    );
    record_err(errors, &koin.probe_error);

    let ktor = try_query(
        &current.android.ktor,
        || query::maven_central_latest_stable("io.ktor", "ktor-bom"),
        "maven central: ktor-bom",
    );
    record_err(errors, &ktor.probe_error);

    // Gradle tracks AGP compatibility. We don't auto-bump it here --
    // the compatibility table moves in discrete steps and a bad bump
    // breaks the rust-android-gradle plugin. A human owns this pin.
    let gradle = current.android.gradle.clone();

    // Tooling block -------------------------------------------------
    let cargo_deny = try_query(
        &current.tooling.cargo_deny,
        || query::crates_io_latest_stable("cargo-deny"),
        "crates.io: cargo-deny",
    );
    record_err(errors, &cargo_deny.probe_error);

    let cargo_vet = try_query(
        &current.tooling.cargo_vet,
        || query::crates_io_latest_stable("cargo-vet"),
        "crates.io: cargo-vet",
    );
    record_err(errors, &cargo_vet.probe_error);

    let xcodegen = try_query(
        &current.tooling.xcodegen,
        || query::github_latest_release("yonaskolb", "XcodeGen"),
        "github: yonaskolb/XcodeGen",
    );
    record_err(errors, &xcodegen.probe_error);

    Versions {
        crux: Crux {
            crux_core: crux_core.value,
            crux_http: crux_http.value,
            crux_kv: crux_kv.value,
            crux_time: crux_time.value,
            crux_platform: crux_platform.value,
            facet: facet.value,
            facet_generate: facet_generate.value,
            serde: serde.value,
            serde_json: serde_json.value,
            uniffi: uniffi.value,
            cargo_swift: cargo_swift.value,
        },
        android: Android {
            compose_bom: compose_bom.value,
            koin: koin.value,
            ktor: ktor.value,
            kotlin: kotlin.value,
            agp: agp.value,
            gradle,
            ndk: current.android.ndk.clone(),
        },
        ios: Ios {},
        tooling: Tooling {
            cargo_deny: cargo_deny.value,
            cargo_vet: cargo_vet.value,
            xcodegen: xcodegen.value,
        },
    }
}

fn record_err(errors: &mut Vec<String>, probe_error: &Option<String>) {
    if let Some(e) = probe_error {
        errors.push(e.clone());
    }
}

#[derive(Debug, serde::Serialize)]
struct Change {
    key: String,
    current: String,
    proposed: String,
}

#[derive(Debug, serde::Serialize)]
struct Unchanged {
    key: String,
    value: String,
}

/// Keys-and-values walker for a `Versions`. Returns pairs in a stable,
/// human-meaningful order (matches the embedded TOML layout) so diff
/// output is deterministic.
fn pairs(v: &Versions) -> Vec<(&'static str, String)> {
    vec![
        ("crux.crux_core", v.crux.crux_core.clone()),
        ("crux.crux_http", v.crux.crux_http.clone()),
        ("crux.crux_kv", v.crux.crux_kv.clone()),
        ("crux.crux_time", v.crux.crux_time.clone()),
        ("crux.crux_platform", v.crux.crux_platform.clone()),
        ("crux.facet", v.crux.facet.clone()),
        ("crux.facet_generate", v.crux.facet_generate.clone()),
        ("crux.serde", v.crux.serde.clone()),
        ("crux.serde_json", v.crux.serde_json.clone()),
        ("crux.uniffi", v.crux.uniffi.clone()),
        ("crux.cargo_swift", v.crux.cargo_swift.clone()),
        ("android.compose_bom", v.android.compose_bom.clone()),
        ("android.koin", v.android.koin.clone()),
        ("android.ktor", v.android.ktor.clone()),
        ("android.kotlin", v.android.kotlin.clone()),
        ("android.agp", v.android.agp.clone()),
        ("android.gradle", v.android.gradle.clone()),
        ("tooling.cargo_deny", v.tooling.cargo_deny.clone()),
        ("tooling.cargo_vet", v.tooling.cargo_vet.clone()),
        ("tooling.xcodegen", v.tooling.xcodegen.clone()),
    ]
}

fn diff_fields(current: &Versions, proposed: &Versions) -> Vec<Change> {
    let cur = pairs(current);
    let pro = pairs(proposed);
    cur.into_iter()
        .zip(pro)
        .filter_map(|((k, c), (_, p))| {
            if c != p {
                Some(Change {
                    key: k.to_string(),
                    current: c,
                    proposed: p,
                })
            } else {
                None
            }
        })
        .collect()
}

fn unchanged_fields(current: &Versions, proposed: &Versions) -> Vec<Unchanged> {
    let cur = pairs(current);
    let pro = pairs(proposed);
    cur.into_iter()
        .zip(pro)
        .filter_map(|((k, c), (_, p))| {
            if c == p {
                Some(Unchanged {
                    key: k.to_string(),
                    value: c,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Write `contents` to `target` atomically (write to a sibling `.tmp`,
/// then `rename`). Creates parent directories as needed.
fn write_atomic(target: &Path, contents: &str) -> Result<(), VectisError> {
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = target.with_extension("toml.tmp");
    fs::write(&tmp, contents)?;
    fs::rename(&tmp, target)?;
    Ok(())
}

/// Scratch-dir root for the verify matrix. Sits inside the user's
/// cache dir (same rationale as `verify::VerifyCache` -- macOS rotates
/// `/tmp` aggressively and a cap-matrix run can take multiple
/// minutes).
fn verify_scratch_dir(pid: u32) -> PathBuf {
    match std::env::var_os("HOME") {
        Some(home) => PathBuf::from(home)
            .join(".cache")
            .join("vectis")
            .join(format!("update-versions-{pid}")),
        None => std::env::temp_dir().join(format!("vectis-update-versions-{pid}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn baseline() -> Versions {
        Versions::embedded().unwrap()
    }

    #[test]
    fn diff_is_empty_when_current_equals_proposed() {
        let v = baseline();
        let changes = diff_fields(&v, &v);
        assert!(changes.is_empty(), "no changes expected; got {changes:?}");
    }

    #[test]
    fn diff_reports_only_changed_fields_and_keeps_key_order() {
        let current = baseline();
        let mut proposed = baseline();
        proposed.crux.crux_core = "0.99.0".to_string();
        proposed.android.kotlin = "9.9.9".to_string();
        let changes = diff_fields(&current, &proposed);
        assert_eq!(changes.len(), 2);
        assert_eq!(changes[0].key, "crux.crux_core");
        assert_eq!(changes[0].current, current.crux.crux_core);
        assert_eq!(changes[0].proposed, "0.99.0");
        assert_eq!(changes[1].key, "android.kotlin");
        assert_eq!(changes[1].proposed, "9.9.9");
    }

    #[test]
    fn unchanged_complements_changes_to_full_key_set() {
        let current = baseline();
        let mut proposed = baseline();
        proposed.crux.crux_http = "0.99.0".to_string();
        let changes = diff_fields(&current, &proposed);
        let unchanged = unchanged_fields(&current, &proposed);
        assert_eq!(changes.len() + unchanged.len(), pairs(&current).len());
    }

    #[test]
    fn resolve_write_target_honours_explicit_override() {
        let explicit = PathBuf::from("/tmp/explicit.toml");
        let resolved = resolve_write_target(Some(&explicit)).unwrap();
        assert_eq!(resolved, explicit);
    }

    #[test]
    fn read_current_pins_falls_back_to_embedded_when_missing() {
        // Use a path that almost certainly does not exist on a real
        // machine; read_current_pins must fall back, not error.
        let missing = PathBuf::from("/nonexistent/definitely/not/a/real/path.toml");
        let v = read_current_pins(&missing).unwrap();
        assert_eq!(v, Versions::embedded().unwrap());
    }

    #[test]
    fn write_atomic_creates_parent_dirs_and_leaves_no_tmp_behind() {
        let scratch = std::env::temp_dir().join(format!(
            "vectis-write-atomic-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let target = scratch.join("nested").join("dir").join("versions.toml");
        write_atomic(&target, "[crux]\ncrux_core = \"x\"\n").unwrap();
        assert!(target.is_file(), "target should exist");
        let tmp = target.with_extension("toml.tmp");
        assert!(!tmp.exists(), "tmp should have been renamed over target");
        let back = fs::read_to_string(&target).unwrap();
        assert!(back.contains("crux_core"), "unexpected contents: {back}");
        let _ = fs::remove_dir_all(&scratch);
    }

    #[test]
    fn write_atomic_replaces_existing_file_without_partial_writes() {
        let scratch = std::env::temp_dir().join(format!(
            "vectis-write-atomic-overwrite-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let target = scratch.join("versions.toml");
        fs::create_dir_all(&scratch).unwrap();
        fs::write(&target, "original").unwrap();
        write_atomic(&target, "[tooling]\nxcodegen = \"9.9.9\"\n").unwrap();
        let back = fs::read_to_string(&target).unwrap();
        assert!(back.contains("xcodegen"));
        assert!(!back.contains("original"));
        let _ = fs::remove_dir_all(&scratch);
    }
}

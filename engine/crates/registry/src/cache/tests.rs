use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use super::*;
use crate::manifest::{Axis, ExtensionPermissions, ExtensionScope};
use crate::test_support::{EnvGuard, cache_env, env_lock, scratch_dir};

fn project_scope() -> ExtensionScope {
    ExtensionScope::Project {
        project_name: "demo".to_string(),
    }
}

fn plugin_target_scope() -> ExtensionScope {
    ExtensionScope::Plugin {
        axis: Axis::Target,
        plugin_slug: "contracts".to_string(),
        capability_dir: PathBuf::from("/adapters/contracts"),
    }
}

fn fixed_sidecar(scope: &ExtensionScope, name: &str, version: &str, source: &str) -> Sidecar {
    Sidecar {
        schema_version: SIDECAR_SCHEMA_VERSION,
        scope: scope_segment(scope).expect("scope segment"),
        tool_name: name.to_string(),
        tool_version: version.to_string(),
        source: source.to_string(),
        fetched_at: "2026-05-07T00:00:00Z".parse().expect("fixed test stamp"),
        permissions_snapshot: ExtensionPermissions {
            read: vec!["$PROJECT_DIR/contracts".to_string()],
            write: Vec::new(),
        },
        sha256: Some(
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string(),
        ),
        package: None,
    }
}

fn write_cached_version(
    scope: &ExtensionScope, name: &str, version: &str, source: &str,
) -> PathBuf {
    let dir = tool_dir(scope, name, version).expect("tool dir");
    fs::create_dir_all(&dir).expect("create version dir");
    fs::write(dir.join(MODULE_FILENAME), b"wasm").expect("write module");
    write_sidecar(&dir.join(SIDECAR_FILENAME), &fixed_sidecar(scope, name, version, source))
        .expect("write sidecar");
    dir
}

#[test]
fn cache_root_honours_override_precedence() {
    let override_dir = scratch_dir("override");
    let xdg_dir = scratch_dir("xdg");
    let home_dir = scratch_dir("home");
    let _g = env_lock();
    let _cache = EnvGuard::scoped("SPECIFY_EXTENSIONS_CACHE", Some(&override_dir));
    let _xdg = EnvGuard::scoped("XDG_CACHE_HOME", Some(&xdg_dir));
    let _home = EnvGuard::scoped("HOME", Some(&home_dir));
    assert_eq!(root().expect("cache root"), override_dir);
}

#[test]
fn cache_root_prefers_xdg() {
    let xdg_dir = scratch_dir("xdg-only");
    let home_dir = scratch_dir("home-only");
    let _g = env_lock();
    let _cache = EnvGuard::scoped("SPECIFY_EXTENSIONS_CACHE", None);
    let _xdg = EnvGuard::scoped("XDG_CACHE_HOME", Some(&xdg_dir));
    let _home = EnvGuard::scoped("HOME", Some(&home_dir));
    assert_eq!(root().expect("cache root"), xdg_dir.join("specify").join("tools"));
}

#[test]
fn cache_root_falls_back_home() {
    let home_dir = scratch_dir("home-fallback");
    let _g = env_lock();
    let _cache = EnvGuard::scoped("SPECIFY_EXTENSIONS_CACHE", None);
    let _xdg = EnvGuard::scoped("XDG_CACHE_HOME", None);
    let _home = EnvGuard::scoped("HOME", Some(&home_dir));
    assert_eq!(root().expect("cache root"), home_dir.join(".cache").join("specify").join("tools"));
}

#[test]
fn scope_segment_rejects_empty() {
    assert_eq!(scope_segment(&project_scope()).expect("project segment"), "project--demo");
    assert_eq!(
        scope_segment(&plugin_target_scope()).expect("plugin segment"),
        "adapter--target--contracts"
    );
    let empty = ExtensionScope::Project {
        project_name: String::new(),
    };
    assert!(matches!(
        scope_segment(&empty),
        Err(ExtensionError::Diag {
            code: "tool-resolver",
            ..
        })
    ));
}

#[test]
fn sidecar_round_trips_rejects_invalid() {
    let root = scratch_dir("sidecar");
    let path = root.join(SIDECAR_FILENAME);
    let sidecar =
        fixed_sidecar(&project_scope(), "contract", "1.0.0", "https://example.test/contract.wasm");

    write_sidecar(&path, &sidecar).expect("write sidecar");
    assert_eq!(read_sidecar(&path).expect("read sidecar"), Some(sidecar));

    fs::write(
        &path,
        "schema-version: 2\nscope: project--demo\ntool-name: contract\ntool-version: 1.0.0\nsource: https://example.test/contract.wasm\nfetched-at: 2026-05-07T00:00:00Z\npermissions-snapshot:\n  read: []\n  write: []\n",
    )
    .expect("write invalid sidecar");
    assert!(matches!(
        read_sidecar(&path),
        Err(ExtensionError::Diag {
            code: "tool-sidecar-schema",
            ..
        })
    ));

    let schema: serde_json::Value =
        serde_json::from_str(EXTENSION_SIDECAR_JSON_SCHEMA).expect("sidecar schema parses");
    jsonschema::validator_for(&schema).expect("sidecar schema compiles");
}

#[test]
fn cache_status_distinguishes_states() {
    let cache_dir = scratch_dir("status-cache");
    let _env = cache_env(&cache_dir);
    assert_eq!(
        status(
            &project_scope(),
            "contract",
            "1.0.0",
            "https://example.test/contract.wasm",
            Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
        )
        .expect("cold status"),
        Status::MissNotFound
    );
    write_cached_version(
        &project_scope(),
        "contract",
        "1.0.0",
        "https://example.test/contract.wasm",
    );
    assert_eq!(
        status(
            &project_scope(),
            "contract",
            "1.0.0",
            "https://example.test/contract.wasm",
            Some("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
        )
        .expect("hit status"),
        Status::Hit
    );
    assert_eq!(
        status(
            &project_scope(),
            "contract",
            "1.0.0",
            "https://example.test/contract.wasm",
            Some("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff")
        )
        .expect("changed status"),
        Status::MissChanged
    );
}

#[test]
fn stage_and_install_replaces_existing() {
    let root = scratch_dir("stage");
    let staged = root.join("staged");
    let dest = root.join("cache").join("project--demo").join("contract").join("1.0.0");
    fs::create_dir_all(staged.join("nested")).expect("create staged");
    fs::write(staged.join(MODULE_FILENAME), b"new").expect("write module");
    fs::write(staged.join("nested").join("probe.txt"), b"probe").expect("write nested");

    let manual_partial = dest.with_extension("manual-tmp");
    fs::create_dir_all(&manual_partial).expect("create manual temp");
    fs::write(manual_partial.join(MODULE_FILENAME), b"partial").expect("write partial");
    assert!(!dest.exists(), "manual sibling staging must not expose dest");
    fs::remove_dir_all(&manual_partial).expect("remove manual temp");

    stage_and_install(&staged, &dest).expect("install staged");
    assert_eq!(fs::read(dest.join(MODULE_FILENAME)).expect("read module"), b"new");
    assert_eq!(fs::read(dest.join("nested").join("probe.txt")).expect("read nested"), b"probe");

    let staged_replacement = root.join("staged-replacement");
    fs::create_dir_all(&staged_replacement).expect("create replacement");
    fs::write(staged_replacement.join(MODULE_FILENAME), b"replacement").expect("write replacement");
    stage_and_install(&staged_replacement, &dest).expect("replace staged");
    assert_eq!(fs::read(dest.join(MODULE_FILENAME)).expect("read replacement"), b"replacement");
    assert!(!dest.join("nested").exists(), "replacement removes old tree");
}

#[test]
fn scan_for_gc_isolates_scope() {
    let cache_dir = scratch_dir("gc-cache");
    let _env = cache_env(&cache_dir);

    let kept_project = write_cached_version(
        &project_scope(),
        "contract",
        "1.0.0",
        "https://example.test/contract.wasm",
    );
    let stale_project = write_cached_version(
        &project_scope(),
        "contract",
        "1.1.0",
        "https://example.test/contract-new.wasm",
    );
    let stale_adapter = write_cached_version(
        &plugin_target_scope(),
        "contract",
        "1.0.0",
        "https://example.test/contract.wasm",
    );

    let kept = HashSet::from([(
        "contract".to_string(),
        "1.0.0".to_string(),
        "https://example.test/contract.wasm".to_string(),
    )]);

    let project_gc = scan_for_gc(&project_scope(), &kept).expect("project gc");
    assert_eq!(project_gc, vec![stale_project]);
    assert!(kept_project.exists());

    let adapter_gc = scan_for_gc(&plugin_target_scope(), &HashSet::new()).expect("adapter gc");
    assert_eq!(adapter_gc, vec![stale_adapter]);
}

// `tool_dir` segments become literal cache path components, so a name
// or version carrying a separator or `..` is a path-traversal vector.
// `validate_segment` must reject each before the path is ever joined.
#[test]
fn tool_dir_rejects_traversal_segments() {
    let scope = project_scope();
    let traversal_cases = ["..", ".", "a/b", "a\\b", ""];
    for bad in traversal_cases {
        assert!(
            matches!(
                tool_dir(&scope, bad, "1.0.0"),
                Err(ExtensionError::Diag {
                    code: "tool-resolver",
                    ..
                })
            ),
            "name `{bad}` must be rejected"
        );
        assert!(
            matches!(
                tool_dir(&scope, "contract", bad),
                Err(ExtensionError::Diag {
                    code: "tool-resolver",
                    ..
                })
            ),
            "version `{bad}` must be rejected"
        );
    }
}

// The cache-root precedence resolves from env vars; the *error* arms
// (empty / relative override, no fallback at all) are easy to break
// when reordering the precedence ladder. Pin each rejection.
#[test]
fn cache_root_rejects_unusable_env() {
    fn rejected(context: &str) {
        assert!(
            matches!(
                root(),
                Err(ExtensionError::Diag {
                    code: "tool-cache-root",
                    ..
                })
            ),
            "{context}"
        );
    }

    let _g = env_lock();

    let relative = EnvGuard::scoped("SPECIFY_EXTENSIONS_CACHE", Some(Path::new("relative/dir")));
    rejected("a relative override must be rejected");
    drop(relative);

    let empty = EnvGuard::scoped("SPECIFY_EXTENSIONS_CACHE", Some(Path::new("")));
    rejected("an empty override must be rejected");
    drop(empty);

    // The remaining cases clear the explicit override so the XDG / HOME
    // fallbacks are the ones under test.
    let _cache = EnvGuard::scoped("SPECIFY_EXTENSIONS_CACHE", None);
    let _xdg = EnvGuard::scoped("XDG_CACHE_HOME", None);

    let relative_home = EnvGuard::scoped("HOME", Some(Path::new("relative-home")));
    rejected("a relative HOME fallback must be rejected");
    drop(relative_home);

    let _home = EnvGuard::scoped("HOME", None);
    rejected("no env source at all must be rejected");
}

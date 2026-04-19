//! Core assembly scaffolding for `vectis init`.
//!
//! Chunk 5 landed the render-only baseline; chunk 6 wires the `--caps`
//! flag through so the handler honours every combination of `http`,
//! `kv`, `time`, `platform`, and `sse`. Inputs are validated, the
//! placeholder map is built from chunk-4's resolved version pins, the
//! function refuses to overwrite any pre-existing target file, then
//! writes every embedded template under the project directory.
//!
//! Chunks 7 / 8 add iOS / Android scaffolds called from the same
//! `init::run` entry point.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::VectisError;
use crate::templates::{Capability, core, render};
use crate::versions::Versions;

/// Result of a successful core scaffold.
///
/// `files` lists target paths in the order they were written -- which is
/// the order the embedded `TEMPLATES` slice declares (chunk-3a MANIFEST §
/// Path mapping order, also matches the RFC's example output).
#[derive(Debug)]
pub struct CoreScaffold {
    pub files: Vec<String>,
}

/// Validate `app_name` against the PascalCase pattern documented in
/// RFC-6 § CLI Surface § `vectis init`.
///
/// A name is valid iff:
/// - The first character is an ASCII uppercase letter.
/// - Every other character is an ASCII alphanumeric.
/// - The name is non-empty.
///
/// Examples (RFC): `Counter`, `TodoApp`, `NoteEditor`. Rejects `counter`,
/// `Todo App` (whitespace), `Todo-App` (hyphen), `123App` (leading digit),
/// `_App` (leading underscore). The constraint matches the way the name
/// is used downstream: a literal Rust struct identifier in `app.rs` /
/// `ffi.rs` / `codegen.rs`.
pub fn validate_app_name(app_name: &str) -> Result<(), VectisError> {
    let mut chars = app_name.chars();
    let first = chars.next().ok_or_else(|| VectisError::InvalidProject {
        message: "app name must not be empty".into(),
    })?;
    if !first.is_ascii_uppercase() {
        return Err(VectisError::InvalidProject {
            message: format!(
                "app name {app_name:?} must start with an ASCII uppercase letter (PascalCase, e.g. \"Counter\")"
            ),
        });
    }
    for c in chars {
        if !c.is_ascii_alphanumeric() {
            return Err(VectisError::InvalidProject {
                message: format!(
                    "app name {app_name:?} must contain only ASCII alphanumeric characters (PascalCase)"
                ),
            });
        }
    }
    Ok(())
}

/// Compute the default Android package per RFC § CLI Surface § `vectis init`:
/// `com.vectis.<lower app name>`.
///
/// Always called (even for core-only / iOS-only scaffolds) because the
/// chunk-3a `codegen.rs` template uses `__ANDROID_PACKAGE__` as the Kotlin
/// namespace -- the binary still has to compile when no Android shell is
/// requested. See chunk-3a MANIFEST § Android-only placeholder.
pub fn default_android_package(app_name: &str) -> String {
    format!("com.vectis.{}", app_name.to_lowercase())
}

/// Render and write the core templates under `project_dir`.
///
/// Atomic refusal: the function walks every target path *first* and
/// returns `InvalidProject` if any of them already exist before any
/// directory is created or any byte is written. This avoids the
/// half-scaffolded-project failure mode the RFC's "one command, working
/// project" promise rules out.
///
/// `caps` selects which capability-marked regions of the templates are
/// kept. Chunk 6 wires this through from `--caps`; pass `&[]` for the
/// render-only baseline.
pub fn scaffold(
    project_dir: &Path,
    app_name: &str,
    android_package: &str,
    versions: &Versions,
    caps: &[Capability],
) -> Result<CoreScaffold, VectisError> {
    validate_app_name(app_name)?;

    let mut planned: Vec<(PathBuf, &'static str, String)> =
        Vec::with_capacity(core::TEMPLATES.len());

    let params = super::build_params(app_name, android_package, versions);

    for entry in core::TEMPLATES {
        let target = project_dir.join(entry.target);
        if target.exists() {
            return Err(VectisError::InvalidProject {
                message: format!(
                    "refusing to overwrite existing file at {} (run `vectis init` against an empty directory)",
                    target.display()
                ),
            });
        }
        let rendered = render(entry.contents, &params, caps);
        planned.push((target, entry.target, rendered));
    }

    // Create the project directory itself if missing -- the user can run
    // `vectis init Counter --dir /tmp/scratch/new-project` and expect
    // `new-project` to come into being.
    if !project_dir.exists() {
        fs::create_dir_all(project_dir)?;
    }

    let mut written = Vec::with_capacity(planned.len());
    for (path, target_str, contents) in planned {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&path, contents)?;
        written.push(target_str.to_string());
    }

    Ok(CoreScaffold { files: written })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn scratch_dir(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!(
            "vectis-init-{label}-{}-{nanos}-{n}",
            std::process::id(),
        ));
        // Deliberately do not create the dir -- some tests want to prove
        // scaffold() will create it itself.
        dir
    }

    fn embedded_versions() -> Versions {
        Versions::embedded().expect("embedded defaults must parse")
    }

    #[test]
    fn rejects_empty_app_name() {
        let err = validate_app_name("").expect_err("empty name must fail");
        match err {
            VectisError::InvalidProject { message } => assert!(message.contains("must not be empty")),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn rejects_lowercase_first_char() {
        for bad in ["counter", "tODO", "_App", "123App", " App"] {
            let err = validate_app_name(bad).expect_err("must reject");
            match err {
                VectisError::InvalidProject { .. } => {}
                other => panic!("unexpected for {bad:?}: {other:?}"),
            }
        }
    }

    #[test]
    fn rejects_non_ascii_alphanumeric() {
        for bad in ["Todo App", "Todo-App", "Todo_App", "Café"] {
            let err = validate_app_name(bad).expect_err("must reject");
            match err {
                VectisError::InvalidProject { .. } => {}
                other => panic!("unexpected for {bad:?}: {other:?}"),
            }
        }
    }

    #[test]
    fn accepts_rfc_examples() {
        for good in ["Counter", "TodoApp", "NoteEditor", "X", "App42"] {
            validate_app_name(good).unwrap_or_else(|_| panic!("must accept {good:?}"));
        }
    }

    #[test]
    fn default_android_package_lowercases_app_name() {
        assert_eq!(default_android_package("Counter"), "com.vectis.counter");
        assert_eq!(default_android_package("TodoApp"), "com.vectis.todoapp");
    }

    #[test]
    fn scaffold_creates_every_core_file_and_creates_missing_project_dir() {
        let dir = scratch_dir("create-dir");
        assert!(!dir.exists());
        let result = scaffold(
            &dir,
            "Counter",
            &default_android_package("Counter"),
            &embedded_versions(),
            &[],
        )
        .expect("scaffold must succeed");
        assert_eq!(result.files.len(), core::TEMPLATES.len());
        for entry in core::TEMPLATES {
            assert!(
                dir.join(entry.target).is_file(),
                "missing rendered file: {}",
                entry.target
            );
        }
    }

    #[test]
    fn scaffold_substitutes_placeholders_in_rendered_files() {
        let dir = scratch_dir("substitute");
        scaffold(
            &dir,
            "Counter",
            &default_android_package("Counter"),
            &embedded_versions(),
            &[],
        )
        .unwrap();
        let app_rs = fs::read_to_string(dir.join("shared/src/app.rs")).unwrap();
        assert!(
            !app_rs.contains("__APP_STRUCT__"),
            "placeholder still present in app.rs"
        );
        assert!(
            app_rs.contains("Hello from Counter"),
            "expected substituted message in app.rs"
        );
        let codegen = fs::read_to_string(dir.join("shared/src/bin/codegen.rs")).unwrap();
        assert!(
            !codegen.contains("__ANDROID_PACKAGE__"),
            "placeholder still present in codegen.rs"
        );
        assert!(
            codegen.contains("com.vectis.counter"),
            "expected default android package in codegen.rs"
        );
    }

    #[test]
    fn scaffold_strips_cap_blocks_for_render_only() {
        let dir = scratch_dir("render-only");
        scaffold(
            &dir,
            "Counter",
            &default_android_package("Counter"),
            &embedded_versions(),
            &[],
        )
        .unwrap();
        for entry in core::TEMPLATES {
            let body = fs::read_to_string(dir.join(entry.target)).unwrap();
            assert!(
                !body.contains("<<<CAP:"),
                "leftover open marker in {}",
                entry.target
            );
            assert!(
                !body.contains("CAP:") || !body.contains(">>>"),
                "leftover close marker in {}",
                entry.target
            );
        }
    }

    #[test]
    fn scaffold_includes_selected_capability_blocks() {
        let dir = scratch_dir("with-caps");
        scaffold(
            &dir,
            "Counter",
            &default_android_package("Counter"),
            &embedded_versions(),
            &[Capability::Http, Capability::Kv, Capability::Sse],
        )
        .unwrap();

        let workspace = fs::read_to_string(dir.join("Cargo.toml")).unwrap();
        assert!(
            workspace.contains("crux_http = \""),
            "workspace dep missing for http: {workspace}"
        );
        assert!(
            workspace.contains("crux_kv = \""),
            "workspace dep missing for kv: {workspace}"
        );
        assert!(
            !workspace.contains("crux_time"),
            "time dep leaked when not requested: {workspace}"
        );

        let app_rs = fs::read_to_string(dir.join("shared/src/app.rs")).unwrap();
        assert!(
            app_rs.contains("FetchData"),
            "http event missing from app.rs"
        );
        assert!(app_rs.contains("LoadData"), "kv event missing from app.rs");
        assert!(
            !app_rs.contains("PlatformRequest"),
            "platform leaked when not requested"
        );

        let shared_cargo = fs::read_to_string(dir.join("shared/Cargo.toml")).unwrap();
        assert!(
            shared_cargo.contains("async-sse"),
            "sse dep missing from shared Cargo.toml"
        );

        // Cap markers must never survive into rendered output, regardless
        // of whether the cap was selected or stripped.
        for entry in core::TEMPLATES {
            let body = fs::read_to_string(dir.join(entry.target)).unwrap();
            assert!(
                !body.contains("<<<CAP:"),
                "leftover open marker in {} with caps",
                entry.target
            );
        }
    }

    #[test]
    fn scaffold_refuses_to_overwrite_existing_files() {
        let dir = scratch_dir("no-overwrite");
        fs::create_dir_all(&dir).unwrap();
        // Plant a file that collides with one of the templates.
        fs::write(dir.join("Cargo.toml"), "pre-existing").unwrap();

        let err = scaffold(
            &dir,
            "Counter",
            &default_android_package("Counter"),
            &embedded_versions(),
            &[],
        )
        .expect_err("scaffold must refuse to overwrite");
        match err {
            VectisError::InvalidProject { message } => {
                assert!(message.contains("refusing to overwrite"));
                assert!(message.contains("Cargo.toml"));
            }
            other => panic!("unexpected: {other:?}"),
        }
        // Crucially, no other files should have been written.
        assert!(!dir.join("shared/src/app.rs").exists());
        // And the pre-existing file is untouched.
        assert_eq!(
            fs::read_to_string(dir.join("Cargo.toml")).unwrap(),
            "pre-existing"
        );
    }

    #[test]
    fn scaffold_rejects_invalid_app_name_before_writing() {
        let dir = scratch_dir("bad-name");
        let err = scaffold(
            &dir,
            "counter",
            &default_android_package("counter"),
            &embedded_versions(),
            &[],
        )
        .expect_err("invalid name must fail");
        match err {
            VectisError::InvalidProject { .. } => {}
            other => panic!("unexpected: {other:?}"),
        }
        // Validation happens before any directory is created.
        assert!(!dir.exists());
    }
}

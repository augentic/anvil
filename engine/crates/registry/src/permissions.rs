//! Permission substitution, canonicalisation, and escape checks for tool runs.

use std::path::{Component, Path, PathBuf};

use crate::error::ExtensionError;
use crate::manifest::looks_like_windows_absolute;

const PROJECT_DIR_VAR: &str = "PROJECT_DIR";
const CAPABILITY_DIR_VAR: &str = "CAPABILITY_DIR";
const LIFECYCLE_RULE_ID: &str = "tool.lifecycle-state-write-denied";

/// Substitute the permission variables (`$PROJECT_DIR`, `$CAPABILITY_DIR`) in one manifest permission entry.
///
/// # Errors
///
/// Returns an error when the template references an unsupported variable,
/// references `$CAPABILITY_DIR` outside plugin scope, contains a parent
/// segment, expands to a relative path, or uses a non-UTF-8 root path.
pub fn substitute(
    template: &str, project_dir: &Path, capability_dir: Option<&Path>,
) -> Result<String, ExtensionError> {
    if has_parent_segment(template) {
        return Err(ExtensionError::invalid_permission(
            template,
            "permission paths must not contain `..` segments",
        ));
    }

    for variable in variables(template)? {
        match variable.as_str() {
            PROJECT_DIR_VAR => {}
            CAPABILITY_DIR_VAR if capability_dir.is_some() => {}
            CAPABILITY_DIR_VAR => {
                return Err(ExtensionError::invalid_permission(
                    template,
                    "$CAPABILITY_DIR is only available to plugin-scope tools",
                ));
            }
            _ => {
                return Err(ExtensionError::invalid_permission(
                    template,
                    format!("unsupported variable `${variable}`"),
                ));
            }
        }
    }

    let project = utf8_path(project_dir, "$PROJECT_DIR", template)?;
    let mut expanded = template.replace("$PROJECT_DIR", project);
    if let Some(capability_dir) = capability_dir {
        let adapter = utf8_path(capability_dir, "$CAPABILITY_DIR", template)?;
        expanded = expanded.replace("$CAPABILITY_DIR", adapter);
    }

    if has_parent_segment(&expanded) {
        return Err(ExtensionError::invalid_permission(
            template,
            "expanded permission path must not contain `..` segments",
        ));
    }
    if !path_is_absolute(&expanded) {
        return Err(ExtensionError::invalid_permission(
            template,
            format!("expanded permission path must be absolute: {expanded}"),
        ));
    }

    Ok(expanded)
}

/// Canonicalise a target path and require it to remain inside an allowed root.
///
/// The target and roots must already exist. Canonicalisation follows symlinks,
/// so a symlink that textually lives under an allowed root but points elsewhere
/// is rejected.
///
/// # Errors
///
/// Returns an error when the target or roots cannot be canonicalised, or when
/// the canonical target is outside every allowed root.
pub fn canonicalise_under(
    target: &Path, allowed_roots: &[&Path],
) -> Result<PathBuf, ExtensionError> {
    if allowed_roots.is_empty() {
        return Err(ExtensionError::permission_denied(
            target,
            "no allowed roots were supplied for permission canonicalisation",
        ));
    }

    let canonical_target = target.canonicalize().map_err(|err| {
        ExtensionError::permission_denied(
            target,
            format!("permission path must already exist and be canonicalisable: {err}"),
        )
    })?;

    for root in allowed_roots {
        let canonical_root = root.canonicalize().map_err(|err| {
            ExtensionError::permission_denied(
                root,
                format!("allowed root must be canonicalisable: {err}"),
            )
        })?;
        if canonical_target == canonical_root || canonical_target.starts_with(&canonical_root) {
            return Ok(canonical_target);
        }
    }

    Err(ExtensionError::permission_denied(
        canonical_target,
        "canonical permission path escapes PROJECT_DIR/CAPABILITY_DIR",
    ))
}

/// Reject write access to Specify lifecycle state under the project root.
///
/// # Errors
///
/// Returns [`ExtensionError::PermissionDenied`] when `target` is `.specify` or a
/// descendant of `.specify` inside `project_dir`.
pub fn deny_lifecycle_write(target: &Path, project_dir: &Path) -> Result<(), ExtensionError> {
    let canonical_project = project_dir.canonicalize().map_err(|err| {
        ExtensionError::permission_denied(
            project_dir,
            format!("project root must be canonicalisable: {err}"),
        )
    })?;
    let lifecycle_root = canonical_project.join(".specify");
    if target == lifecycle_root || target.starts_with(&lifecycle_root) {
        return Err(ExtensionError::permission_denied(target, LIFECYCLE_RULE_ID));
    }
    Ok(())
}

fn variables(template: &str) -> Result<Vec<String>, ExtensionError> {
    let mut variables = Vec::new();
    let mut chars = template.char_indices().peekable();
    while let Some((_, ch)) = chars.next() {
        if ch != '$' {
            continue;
        }
        let mut name = String::new();
        while let Some((_, next)) = chars.peek().copied() {
            if next == '_' || next.is_ascii_alphanumeric() {
                name.push(next);
                let _ = chars.next();
            } else {
                break;
            }
        }
        if name.is_empty() {
            return Err(ExtensionError::invalid_permission(
                template,
                "permission variables must be named, for example $PROJECT_DIR",
            ));
        }
        variables.push(name);
    }
    Ok(variables)
}

fn utf8_path<'a>(
    path: &'a Path, variable: &str, template: &str,
) -> Result<&'a str, ExtensionError> {
    path.to_str().ok_or_else(|| {
        ExtensionError::invalid_permission(
            template,
            format!("{variable} contains non-UTF-8 bytes and cannot be exposed to WASI"),
        )
    })
}

fn has_parent_segment(value: &str) -> bool {
    value.split(['/', '\\']).any(|segment| segment == "..")
        || Path::new(value).components().any(|component| matches!(component, Component::ParentDir))
}

fn path_is_absolute(value: &str) -> bool {
    Path::new(value).is_absolute() || looks_like_windows_absolute(value)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn substitute_expands_dirs() {
        let project = Path::new("/tmp/project");
        let adapter = Path::new("/tmp/adapter");

        assert_eq!(
            substitute("$PROJECT_DIR/contracts", project, Some(adapter)).expect("project"),
            "/tmp/project/contracts"
        );
        assert_eq!(
            substitute("$CAPABILITY_DIR/templates", project, Some(adapter)).expect("adapter"),
            "/tmp/adapter/templates"
        );
    }

    #[test]
    fn substitute_rejects_out_of_scope() {
        let err = substitute("$CAPABILITY_DIR/templates", Path::new("/tmp/project"), None)
            .expect_err("project-scope capability dir must fail");
        assert!(matches!(err, ExtensionError::InvalidPermission { .. }), "{err}");
        assert!(err.to_string().contains("$CAPABILITY_DIR"), "{err}");
    }

    #[test]
    fn substitute_rejects_bad_variables() {
        for template in ["$HOME/contracts", "$PROJECT_DIR/../contracts", "contracts"] {
            let err = substitute(template, Path::new("/tmp/project"), None)
                .expect_err("invalid template must fail");
            assert!(matches!(err, ExtensionError::InvalidPermission { .. }), "{err}");
        }
    }

    #[test]
    fn canonicalise_rejects_symlink_escape() {
        let tmp = tempdir().expect("tempdir");
        let project = tmp.path().join("project");
        let outside = tmp.path().join("outside");
        fs::create_dir_all(&project).expect("project");
        fs::create_dir_all(&outside).expect("outside");

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, project.join("escape")).expect("symlink");
            let err = canonicalise_under(&project.join("escape"), &[&project])
                .expect_err("symlink escape must fail");
            assert!(matches!(err, ExtensionError::PermissionDenied { .. }), "{err}");
        }
    }

    #[test]
    fn canonicalise_accepts_descendant() {
        let tmp = tempdir().expect("tempdir");
        let project = tmp.path().join("project");
        let contracts = project.join("contracts");
        fs::create_dir_all(&contracts).expect("contracts");

        let canonical = canonicalise_under(&contracts, &[&project]).expect("canonical");
        assert_eq!(canonical, contracts.canonicalize().expect("canonical contracts"));
    }

    #[test]
    fn lifecycle_denies_specify_state() {
        let tmp = tempdir().expect("tempdir");
        let project = tmp.path().join("project");
        let specify = project.join(".specify");
        fs::create_dir_all(&specify).expect("specify");

        let err = deny_lifecycle_write(&specify.canonicalize().expect("canonical"), &project)
            .expect_err("lifecycle write must fail");
        assert!(matches!(err, ExtensionError::PermissionDenied { .. }), "{err}");
        assert!(err.to_string().contains(LIFECYCLE_RULE_ID), "{err}");
    }

    // `canonicalise_under` is the symlink-escape gate. The empty-roots
    // arm is a defensive deny (no roots means nothing is allowed); the
    // multi-root arm must accept a target under the *second* root, not
    // just the first.
    #[test]
    fn canonicalise_root_handling() {
        let tmp = tempdir().expect("tempdir");
        let first = tmp.path().join("first");
        let second = tmp.path().join("second");
        let nested = second.join("nested");
        fs::create_dir_all(&first).expect("first");
        fs::create_dir_all(&nested).expect("nested");

        let no_roots = canonicalise_under(&nested, &[]).expect_err("empty roots must deny");
        assert!(matches!(no_roots, ExtensionError::PermissionDenied { .. }), "{no_roots}");

        let canonical =
            canonicalise_under(&nested, &[first.as_path(), second.as_path()]).expect("second root");
        assert_eq!(canonical, nested.canonicalize().expect("canonical nested"));
    }

    // The `$`-variable scanner has to distinguish a named variable from a
    // bare `$`. A `$` with no following name, and a fully unsupported
    // name, must both fail; multiple supported variables in one template
    // must all expand.
    #[test]
    fn substitute_variable_grammar() {
        let project = Path::new("/tmp/project");
        let adapter = Path::new("/tmp/adapter");

        let bare_dollar = substitute("$/contracts", project, Some(adapter))
            .expect_err("bare `$` is not a named variable");
        assert!(matches!(bare_dollar, ExtensionError::InvalidPermission { .. }), "{bare_dollar}");

        let trailing_dollar =
            substitute("$PROJECT_DIR/sub$", project, Some(adapter)).expect_err("trailing bare `$`");
        assert!(
            matches!(trailing_dollar, ExtensionError::InvalidPermission { .. }),
            "{trailing_dollar}"
        );

        assert_eq!(
            substitute("$PROJECT_DIR/$CAPABILITY_DIR-mixed", project, Some(adapter))
                .expect("both variables expand"),
            "/tmp/project//tmp/adapter-mixed"
        );
    }

    // `deny_lifecycle_write` must match `.specify` as a path *component*,
    // not a textual prefix: a sibling like `.specify-data` is a legitimate
    // write target and must NOT be denied, while a descendant of
    // `.specify` must be.
    #[test]
    fn lifecycle_boundary_is_component_wise() {
        let tmp = tempdir().expect("tempdir");
        let project = tmp.path().join("project");
        fs::create_dir_all(&project).expect("project");
        let canonical_project = project.canonicalize().expect("canonical project");

        let sibling = canonical_project.join(".specify-data");
        deny_lifecycle_write(&sibling, &project).expect("sibling of .specify is writable");

        let descendant = canonical_project.join(".specify").join("slices");
        let err = deny_lifecycle_write(&descendant, &project)
            .expect_err("descendant of .specify is denied");
        assert!(err.to_string().contains(LIFECYCLE_RULE_ID), "{err}");
    }
}

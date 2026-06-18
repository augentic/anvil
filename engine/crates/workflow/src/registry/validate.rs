//! Registry-shape validators — invariants that `serde` cannot express
//! (kebab names, unique projects, URL shape, contract-roles
//! consistency). Methods are exposed as `impl Registry` for fluent use.

use std::collections::{HashMap, HashSet};

use specify_error::{Error, is_kebab};

use crate::registry::catalog::{Registry, RegistryProject};

impl Registry {
    /// Enforce invariants that serde cannot express on its own:
    /// `version == 1`, kebab-case project names, non-empty required
    /// strings, unique project names, and well-formed `projects[].url`
    /// values. Returns the first error encountered — the convention used
    /// elsewhere in the registry crate for fast-fail shape validation.
    ///
    /// # Errors
    ///
    /// Returns the first violation as an [`Error::Diag`] whose `code` names
    /// the broken invariant — e.g. `registry-version-unsupported`,
    /// `registry-project-name-empty`, `registry-project-name-not-kebab`,
    /// or a duplicate-name / malformed-`url` discriminant.
    pub fn validate_shape(&self) -> Result<(), Error> {
        if self.version != 1 {
            return Err(Error::Diag {
                code: "registry-version-unsupported",
                detail: format!(
                    "registry.yaml: unsupported version {}; v1 is the only accepted value",
                    self.version
                ),
            });
        }

        let mut seen_names: HashSet<&str> = HashSet::new();
        let mut producers: HashMap<&str, &str> = HashMap::new();
        for (idx, project) in self.projects.iter().enumerate() {
            if project.name.is_empty() {
                return Err(Error::Diag {
                    code: "registry-project-name-empty",
                    detail: format!("registry.yaml: projects[{idx}].name is empty"),
                });
            }
            if !is_kebab(&project.name) {
                return Err(Error::Diag {
                    code: "registry-project-name-not-kebab",
                    detail: format!(
                        "registry.yaml: projects[{idx}].name `{}` must be kebab-case \
                         (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens)",
                        project.name
                    ),
                });
            }
            if project.url.is_empty() {
                return Err(Error::Diag {
                    code: "registry-project-url-empty",
                    detail: format!(
                        "registry.yaml: projects[{idx}] (`{}`).url is empty",
                        project.name
                    ),
                });
            }
            validate_project_url(&project.url, idx, &project.name)?;
            if !seen_names.insert(project.name.as_str()) {
                return Err(Error::Diag {
                    code: "registry-project-name-duplicate",
                    detail: format!("registry.yaml: duplicate project name `{}`", project.name),
                });
            }

            validate_project_contracts(project, &mut producers)?;
            validate_greenfield_seed(project, idx)?;
        }

        // The registry does not author a project's adapter or
        // description for plan-time topology. Those facets live in each
        // project's `project.yaml` and are checked against
        // `.specify/topology.lock` by `specify plan validate`
        // (`topology-cache-stale`).

        Ok(())
    }

    /// Workspace-only shape check.
    ///
    /// Runs the base [`Registry::validate_shape`] first, then layers on
    /// the additional invariant that a **registry-only workspace**
    /// must never list itself as a project: any entry with `url: .` is
    /// rejected with a `workspace-cannot-be-project` diagnostic. The
    /// workspace holds platform-level state (registry, change brief,
    /// plan, workspace slots) but is never a code project.
    ///
    /// Callers opt in by checking `project.yaml:workspace: true` and
    /// invoking this method in addition to (or instead of) the base
    /// [`Registry::validate_shape`]. Non-workspace callers continue to use
    /// the base method unchanged — this is a strictly additive API.
    ///
    /// # Errors
    ///
    /// Returns the first base-shape error if `validate_shape` fails,
    /// or a `workspace-cannot-be-project` config error if any entry's `url`
    /// equals `.`.
    pub fn validate_shape_workspace(&self) -> Result<(), Error> {
        self.validate_shape()?;
        for (idx, project) in self.projects.iter().enumerate() {
            if project.url == "." {
                return Err(Error::Diag {
                    code: "workspace-cannot-be-project",
                    detail: format!(
                        "registry.yaml: projects[{idx}] (`{}`).url is `.`; \
                         a registry-only workspace must not appear in its own \
                         registry — code projects always live in their own repos",
                        project.name
                    ),
                });
            }
        }
        Ok(())
    }
}

/// Enforce contract-roles invariants for a single project: path
/// validity (no absolute or `..` paths), self-consistency (no path
/// appearing in both `produces` and `consumes`), and single-producer
/// registration into the shared `producers` accumulator. Returns the
/// first error encountered.
fn validate_project_contracts<'a>(
    project: &'a RegistryProject, producers: &mut HashMap<&'a str, &'a str>,
) -> Result<(), Error> {
    let Some(roles) = &project.contracts else {
        return Ok(());
    };
    for path in roles.produces.iter().chain(roles.consumes.iter()) {
        if path.starts_with('/') || path.contains("..") {
            return Err(Error::Diag {
                code: "registry-contract-path-not-relative",
                detail: format!(
                    "registry.yaml: contract path '{}' in project '{}' must be relative (no '..' or absolute paths)",
                    path, project.name
                ),
            });
        }
    }
    let produced: HashSet<&str> = roles.produces.iter().map(String::as_str).collect();
    for path in &roles.consumes {
        if produced.contains(path.as_str()) {
            return Err(Error::Diag {
                code: "registry-contract-produces-and-consumes",
                detail: format!(
                    "registry.yaml: project '{}' lists '{}' in both 'produces' and 'consumes'",
                    project.name, path
                ),
            });
        }
    }
    for path in &roles.produces {
        if let Some(existing) = producers.get(path.as_str()) {
            return Err(Error::Diag {
                code: "registry-contract-multiple-producers",
                detail: format!(
                    "registry.yaml: contract path '{}' is produced by both '{}' and '{}'",
                    path, existing, project.name
                ),
            });
        }
        producers.insert(path, &project.name);
    }
    Ok(())
}

/// RFC-46 D6 — enforce that every `greenfield_seed.domains[]` entry is a
/// non-empty kebab-case slug. Domains project into `surface[]` domain
/// directory slugs, so they share the surface's kebab grammar.
fn validate_greenfield_seed(project: &RegistryProject, idx: usize) -> Result<(), Error> {
    let Some(seed) = &project.greenfield_seed else {
        return Ok(());
    };
    for domain in &seed.domains {
        if !is_kebab(domain) {
            return Err(Error::Diag {
                code: "registry-greenfield-seed-domain-not-kebab",
                detail: format!(
                    "registry.yaml: projects[{idx}] (`{}`).greenfield_seed.domains entry `{domain}` \
                     must be kebab-case (lowercase ascii, digits, single hyphens; \
                     no leading/trailing/doubled hyphens)",
                    project.name
                ),
            });
        }
    }
    Ok(())
}

/// Reject malformed `projects[].url` while accepting: `.`,
/// repo-relative paths, `http(s)://`, `git@host:path`, `ssh://`, and
/// `git+http(s)://` / `git+ssh://` forms.
fn validate_project_url(url: &str, idx: usize, project_name: &str) -> Result<(), Error> {
    const ALLOWED_SCHEMES: &[&str] = &["http", "https", "ssh", "git+https", "git+http", "git+ssh"];

    if url.trim().is_empty() {
        return Err(Error::Diag {
            code: "registry-project-url-empty",
            detail: format!(
                "registry.yaml: projects[{idx}] (`{project_name}`).url is empty or whitespace-only"
            ),
        });
    }
    if url != url.trim() {
        return Err(Error::Diag {
            code: "registry-project-url-whitespace",
            detail: format!(
                "registry.yaml: projects[{idx}] (`{project_name}`).url must not have leading or trailing whitespace"
            ),
        });
    }

    if url == "." {
        return Ok(());
    }

    if url.starts_with("git@") {
        return Ok(());
    }

    if let Some(pos) = url.find("://") {
        let scheme = &url[..pos];
        if !ALLOWED_SCHEMES.contains(&scheme) {
            return Err(Error::Diag {
                code: "registry-project-url-scheme-unsupported",
                detail: format!(
                    "registry.yaml: projects[{idx}] (`{project_name}`).url has unsupported URL scheme `{scheme}`: \
                     expected one of http, https, ssh, git+https, git+http, git+ssh, a `git@host:path` remote, `.`, or a relative path"
                ),
            });
        }
        return Ok(());
    }

    if url.contains(':') {
        return Err(Error::Diag {
            code: "registry-project-url-malformed",
            detail: format!(
                "registry.yaml: projects[{idx}] (`{project_name}`).url `{url}` is not valid: \
                 ':' is only allowed in `git@host:path` remotes or in `scheme://` URLs"
            ),
        });
    }

    if url.starts_with('/') {
        return Err(Error::Diag {
            code: "registry-project-url-absolute",
            detail: format!(
                "registry.yaml: projects[{idx}] (`{project_name}`).url must be a relative path, `.`, or a remote URL — absolute filesystem paths are not allowed"
            ),
        });
    }

    #[cfg(windows)]
    if looks_like_windows_drive_path(url) {
        return Err(Error::Diag {
            code: "registry-project-url-absolute",
            detail: format!(
                "registry.yaml: projects[{idx}] (`{project_name}`).url must be a relative path, `.`, or a remote URL — absolute Windows paths are not allowed"
            ),
        });
    }

    Ok(())
}

#[cfg(windows)]
fn looks_like_windows_drive_path(url: &str) -> bool {
    let mut chars = url.chars();
    let Some(c) = chars.next() else {
        return false;
    };
    c.is_ascii_alphabetic() && chars.next() == Some(':')
}

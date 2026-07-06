//! In-process Road B framework checkers (see DECISIONS.md §"Framework
//! lint engine: generic dispatcher (Road A / Road B)").
//!
//! Framework runs have no `project.yaml` to populate a tool inventory,
//! so the first-party framework checkers are declared here as a closed
//! inventory keyed by name. The `kind: tool` evaluator
//! ([`crate::lint::eval::tool`]) resolves a hint's `value` against this
//! inventory first and calls the named checker directly — handing typed
//! [`Diagnostic`] findings straight back with no JSON serialise→reparse
//! round-trip and no [`crate::lint::eval::tool::ToolRunner`] hop. The
//! trait survives only for the genuine project-side WASI path. Name
//! resolution and rule-owned `config:` policy forwarding are unchanged.

mod extension;
mod links_registry;
mod marketplace;
mod prose;
mod rules;
mod scenarios;
mod skill_body;
mod support;

use std::path::Path;

use specify_diagnostics::Diagnostic;

use self::support::ToolFinding;

/// One in-process framework checker: its declared name and entry point.
struct FrameworkChecker {
    name: &'static str,
    run: fn(&Path, &[String]) -> Vec<ToolFinding>,
}

/// Closed inventory of framework checkers the `kind: tool` evaluator
/// resolves by name. Grows one row per Road B family tool.
const FRAMEWORK_CHECKERS: &[FrameworkChecker] = &[
    FrameworkChecker {
        name: "scenarios",
        run: scenarios::run,
    },
    FrameworkChecker {
        name: "skill-body",
        run: skill_body::run,
    },
    FrameworkChecker {
        name: "links-registry",
        run: links_registry::run,
    },
    FrameworkChecker {
        name: "marketplace",
        run: marketplace::run,
    },
    FrameworkChecker {
        name: "prose",
        run: prose::run,
    },
    FrameworkChecker {
        name: "rules",
        run: rules::run,
    },
    FrameworkChecker {
        name: "extension",
        run: extension::run,
    },
];

fn lookup(name: &str) -> Option<&'static FrameworkChecker> {
    FRAMEWORK_CHECKERS.iter().find(|checker| checker.name == name)
}

/// Whether `name` resolves to an in-process framework checker. The
/// `kind: tool` evaluator consults this before falling back to the
/// [`crate::lint::eval::tool::ToolRunner`] WASI path.
#[must_use]
pub fn is_framework_checker(name: &str) -> bool {
    lookup(name).is_some()
}

/// Run the named in-process checker against `project_dir` with the
/// evaluator's positional `args` (candidate path, then the rule's
/// forwarded `config:` JSON), returning typed findings. `None` when
/// `name` is not an in-process checker — the caller then routes the
/// hint through the WASI [`crate::lint::eval::tool::ToolRunner`].
///
/// Findings carry placeholder `id` / `fingerprint`; the evaluator
/// restamps both on fold.
#[must_use]
pub fn run_checker(name: &str, project_dir: &Path, args: &[String]) -> Option<Vec<Diagnostic>> {
    let checker = lookup(name)?;
    let findings = (checker.run)(project_dir, args);
    Some(support::to_diagnostics(&findings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_exactly_the_seven_checkers() {
        for name in [
            "scenarios",
            "skill-body",
            "links-registry",
            "marketplace",
            "prose",
            "rules",
            "extension",
        ] {
            assert!(is_framework_checker(name), "{name} must be declared");
        }
        assert!(!is_framework_checker("agent-teams"), "agent-teams retired with CORE-012");
        assert!(!is_framework_checker("contract"), "adapter tools stay WASI-resolved");
    }

    // The per-checker behaviours (marketplace absent-manifest skip, extension
    // silent-on-no-declarations) live with each checker's own module. What the
    // dispatch tests below pin has no e2e equivalent: the framework-lint engine
    // resolves `kind: tool` hints through `crate::lint::eval::tool`, which calls
    // the checker fn directly — no fixture routes through `run_checker` itself,
    // so its name-resolution arms are CLI-unreachable and stay as unit Keeps.

    #[test]
    fn run_checker_routes_to_named_checker() {
        // The Some-path: a declared name resolves and the checker's findings
        // are handed back as typed diagnostics. An empty tree carries no
        // marketplace, so the checker returns no findings.
        let dir = tempfile::tempdir().expect("tempdir");
        let diagnostics = run_checker("marketplace", dir.path(), &[]).expect("declared checker");
        assert!(diagnostics.is_empty(), "empty tree has no marketplace drift");
    }

    #[test]
    fn unknown_checker_is_none() {
        assert!(run_checker("ghost", Path::new("."), &[]).is_none());
    }
}

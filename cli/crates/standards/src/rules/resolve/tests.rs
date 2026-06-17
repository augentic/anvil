use std::fs;
use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::*;

/// Minimal frontmatter + body that satisfies the CH-11 parser and
/// the codex-rule schema. The shared `id` namespace varies by
/// caller; a 30+ char trigger keeps schema validation happy.
fn rule_markdown(id: &str, title: &str) -> String {
    format!(
        "---\nid: {id}\ntitle: {title}\nseverity: important\ntrigger: Synthetic CH-12 resolver fixture trigger sentence long enough for schema.\n---\n\n## Rule\n\nBody for {id}.\n"
    )
}

fn write_rule(path: &Path, id: &str, title: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, rule_markdown(id, title)).expect("write rule fixture");
}

fn inputs<'a>(
    project_dir: &'a Path, rules_root: Option<&'a Path>, target_adapter: &'a str,
    source_adapters: &'a [String],
) -> ResolveInputs<'a> {
    ResolveInputs {
        project_dir,
        rules_root,
        target_adapter,
        source_adapters,
        artifact_paths: &[],
        languages: &[],
        include_deprecated: false,
        include_unmatched: false,
        include_core: false,
    }
}

fn no_sources() -> Vec<String> {
    Vec::new()
}

const CACHE_ENV: &str = "SPECIFY_PROJECT_CACHE";

/// Restores the previous `SPECIFY_PROJECT_CACHE` value on drop.
struct CacheGuard(Option<std::ffi::OsString>);

impl Drop for CacheGuard {
    #[expect(unsafe_code, reason = "restore the cache-root env var pinned for the test")]
    fn drop(&mut self) {
        // SAFETY: nextest runs each test in its own process, so no other
        // thread observes the env mutation for the guard's lifetime.
        unsafe {
            match self.0.take() {
                Some(prev) => std::env::set_var(CACHE_ENV, prev),
                None => std::env::remove_var(CACHE_ENV),
            }
        }
    }
}

/// Pin the out-of-tree cache root inside `tmp` so the codex / manifest
/// cache the resolver probes is hermetic and auto-cleaned.
#[expect(unsafe_code, reason = "pin the cache-root env var into the test tempdir")]
fn scoped_cache(tmp: &TempDir) -> CacheGuard {
    let prev = std::env::var_os(CACHE_ENV);
    // SAFETY: see `CacheGuard::drop` — single-process test isolation.
    unsafe { std::env::set_var(CACHE_ENV, tmp.path().join("project-cache")) };
    CacheGuard(prev)
}

/// Out-of-tree distributed-codex cache root for `project`.
fn codex_cache(project: &Path) -> PathBuf {
    specify_schema::cache::project_cache_dir(project).join("codex")
}

/// Out-of-tree manifest-mirror cache root for `project`.
fn manifest_cache(project: &Path) -> PathBuf {
    specify_schema::cache::project_cache_dir(project).join("manifests")
}

/// Test 1: shared rules under explicit `--rules-root` flow through
/// as `origin=shared`, `path-root=rules-root`, and the path is
/// relative to the rules root.
#[test]
fn shared_rules_from_explicit_rules_root() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared universal",
    );

    let sources = no_sources();
    let result = resolve(&inputs(project.path(), Some(rules_root.path()), "omnia", &sources))
        .expect("resolve succeeds");

    assert_eq!(result.len(), 1, "exactly one shared rule expected");
    let entry = &result[0];
    assert_eq!(entry.rule.id, "UNI-001");
    assert_eq!(entry.origin, Origin::Shared);
    assert_eq!(entry.path_root, PathRoot::RulesRoot);
    assert_eq!(entry.path, "adapters/shared/rules/universal/uni-001.md");
}

/// Core pack root: rules under
/// `adapters/shared/rules/core/` resolve with `Origin::Core` and
/// `PathRoot::RulesRoot`, alongside any shared-pack rules.
#[test]
fn core_rules_from_explicit_rules_root() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared universal",
    );
    write_rule(
        &rules_root.path().join("adapters/shared/rules/core/CORE-fixture.md"),
        "CORE-001",
        "Core fixture",
    );

    let sources = no_sources();
    let result = resolve(&inputs(project.path(), Some(rules_root.path()), "omnia", &sources))
        .expect("resolve succeeds with core pack");

    let core = result.iter().find(|e| e.rule.id == "CORE-001").expect("core rule present");
    assert_eq!(core.origin, Origin::Core);
    assert_eq!(core.path_root, PathRoot::RulesRoot);
    assert_eq!(core.path, "adapters/shared/rules/core/CORE-fixture.md");

    let shared = result.iter().find(|e| e.rule.id == "UNI-001").expect("shared still present");
    assert_eq!(shared.origin, Origin::Shared);
}

/// Test 2: monorepo / co-located case — no `--rules-root`, but the
/// project tree carries the shared rules. Probe step 2 fires and
/// resolution succeeds with `project_dir` as the rules root.
#[test]
fn rules_root_probe_falls_back() {
    let project = TempDir::new().expect("project");
    write_rule(
        &project.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Monorepo shared",
    );

    let sources = no_sources();
    let result = resolve(&inputs(project.path(), None, "omnia", &sources))
        .expect("resolve succeeds in monorepo layout");

    assert_eq!(result.len(), 1);
    let entry = &result[0];
    assert_eq!(entry.origin, Origin::Shared);
    assert_eq!(entry.path_root, PathRoot::RulesRoot);
    assert_eq!(entry.path, "adapters/shared/rules/universal/uni-001.md");
}

/// Test 3: probe step 4 — no explicit root, no monorepo fallback,
/// no distributed codex cache — must produce the closed
/// `rules-root-required` error.
#[test]
fn rules_root_required_when_no_probe() {
    let project = TempDir::new().expect("project");
    let sources = no_sources();
    let err = resolve(&inputs(project.path(), None, "omnia", &sources)).unwrap_err();
    assert!(matches!(err, ResolveError::RulesRootRequired), "got: {err:?}");
}

/// Probe step 3 (RM-07): with no `--rules-root` and no monorepo
/// tree, the distributed codex cache under the out-of-tree
/// `<project-cache>/codex/` resolves shared rules. The cache root
/// becomes the rules root, so the path is relative to it.
#[test]
fn shared_rules_from_codex_cache() {
    let project = TempDir::new().expect("project");
    let _cache = scoped_cache(&project);
    write_rule(
        &codex_cache(project.path()).join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Distributed codex shared",
    );

    let sources = no_sources();
    let result = resolve(&inputs(project.path(), None, "omnia", &sources))
        .expect("resolve succeeds via the distributed codex cache");

    assert_eq!(result.len(), 1);
    let entry = &result[0];
    assert_eq!(entry.rule.id, "UNI-001");
    assert_eq!(entry.origin, Origin::Shared);
    assert_eq!(entry.path_root, PathRoot::RulesRoot);
    assert_eq!(entry.path, "adapters/shared/rules/universal/uni-001.md");
}

/// Probe precedence: the monorepo tree (step 2) wins over the
/// distributed codex cache (step 3). Only the monorepo rule
/// resolves; the cache tree is never walked.
#[test]
fn monorepo_wins_over_codex_cache() {
    let project = TempDir::new().expect("project");
    let _cache = scoped_cache(&project);
    write_rule(
        &project.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Monorepo shared",
    );
    write_rule(
        &codex_cache(project.path()).join("adapters/shared/rules/universal/uni-002.md"),
        "UNI-002",
        "Cache shared",
    );

    let sources = no_sources();
    let result = resolve(&inputs(project.path(), None, "omnia", &sources))
        .expect("resolve succeeds choosing the monorepo root");

    assert_eq!(result.len(), 1, "only the monorepo tree should be walked");
    assert_eq!(result[0].rule.id, "UNI-001");
}

/// Probe precedence: an explicit `--rules-root` (step 1) wins over
/// a distributed codex cache (step 3).
#[test]
fn explicit_root_wins_over_codex_cache() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    let _cache = scoped_cache(&project);
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Explicit shared",
    );
    write_rule(
        &codex_cache(project.path()).join("adapters/shared/rules/universal/uni-002.md"),
        "UNI-002",
        "Cache shared",
    );

    let sources = no_sources();
    let result = resolve(&inputs(project.path(), Some(rules_root.path()), "omnia", &sources))
        .expect("resolve succeeds choosing the explicit rules root");

    assert_eq!(result.len(), 1, "only the explicit rules root should be walked");
    assert_eq!(result[0].rule.id, "UNI-001");
}

/// Test 4: target overlay resolves from the project-local rung
/// while shared rules continue to flow from the explicit
/// `--rules-root`. The target entry carries `path-root=project-dir`.
#[test]
fn target_overlay_from_project_local() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared universal",
    );
    write_rule(
        &project.path().join("adapters/targets/omnia/rules/omnia-001.md"),
        "OMNIA-001",
        "Omnia overlay",
    );

    let sources = no_sources();
    let result = resolve(&inputs(project.path(), Some(rules_root.path()), "omnia", &sources))
        .expect("resolve succeeds");

    assert_eq!(result.len(), 2);
    let shared = result.iter().find(|e| e.rule.id == "UNI-001").expect("shared present");
    let target = result.iter().find(|e| e.rule.id == "OMNIA-001").expect("target present");
    assert_eq!(shared.origin, Origin::Shared);
    assert_eq!(shared.path_root, PathRoot::RulesRoot);
    assert_eq!(target.origin, Origin::Target);
    assert_eq!(target.path_root, PathRoot::ProjectDir);
    assert_eq!(target.path, "adapters/targets/omnia/rules/omnia-001.md");
}

/// Test 5: rules-root fallback — project-local rung empty, manifest
/// cache empty, explicit `--rules-root` carries the target overlay.
#[test]
fn target_overlay_falls_back_to_rules_root() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared universal",
    );
    write_rule(
        &rules_root.path().join("adapters/targets/omnia/rules/omnia-001.md"),
        "OMNIA-001",
        "Omnia fallback overlay",
    );

    let sources = no_sources();
    let result = resolve(&inputs(project.path(), Some(rules_root.path()), "omnia", &sources))
        .expect("resolve succeeds");

    let target = result.iter().find(|e| e.rule.id == "OMNIA-001").expect("target present");
    assert_eq!(target.origin, Origin::Target);
    assert_eq!(target.path_root, PathRoot::RulesRoot);
    assert_eq!(target.path, "adapters/targets/omnia/rules/omnia-001.md");
}

/// Test 6: source overlay from project-local rung. Confirms
/// `Origin::Source` + `PathRoot::ProjectDir` assignment.
#[test]
fn source_overlay_from_project_local() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared universal",
    );
    write_rule(
        &project.path().join("adapters/sources/typescript/rules/src-001.md"),
        "SRC-001",
        "TS source overlay",
    );

    let sources = vec!["typescript".to_string()];
    let result = resolve(&inputs(project.path(), Some(rules_root.path()), "omnia", &sources))
        .expect("resolve succeeds");

    let src = result.iter().find(|e| e.rule.id == "SRC-001").expect("source present");
    assert_eq!(src.origin, Origin::Source);
    assert_eq!(src.path_root, PathRoot::ProjectDir);
    assert_eq!(src.path, "adapters/sources/typescript/rules/src-001.md");
}

/// Test 7: multiple bound source adapters each contribute their
/// own overlay; both `Source` entries appear in the result.
#[test]
fn multiple_source_overlays() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared universal",
    );
    write_rule(
        &project.path().join("adapters/sources/typescript/rules/src-001.md"),
        "SRC-001",
        "TS overlay",
    );
    write_rule(
        &project.path().join("adapters/sources/documentation/rules/src-002.md"),
        "SRC-002",
        "Docs overlay",
    );

    let sources = vec!["typescript".to_string(), "documentation".to_string()];
    let result = resolve(&inputs(project.path(), Some(rules_root.path()), "omnia", &sources))
        .expect("resolve succeeds");

    let source_entries: Vec<_> = result.iter().filter(|e| e.origin == Origin::Source).collect();
    assert_eq!(source_entries.len(), 2);
    assert!(source_entries.iter().any(|e| e.rule.id == "SRC-001"));
    assert!(source_entries.iter().any(|e| e.rule.id == "SRC-002"));
}

/// Test 8: manifest-cache rung. Project-local missing, manifest
/// cache present — the result carries `PathRoot::Cache` and the
/// cache-relative path starts with `manifests/...`.
#[test]
fn cache_overlay_when_local_missing() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    let _cache = scoped_cache(&project);
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared universal",
    );
    write_rule(
        &manifest_cache(project.path()).join("sources/typescript/rules/src-001.md"),
        "SRC-001",
        "TS cache overlay",
    );

    let sources = vec!["typescript".to_string()];
    let result = resolve(&inputs(project.path(), Some(rules_root.path()), "omnia", &sources))
        .expect("resolve succeeds");

    let src = result.iter().find(|e| e.rule.id == "SRC-001").expect("source present");
    assert_eq!(src.origin, Origin::Source);
    assert_eq!(src.path_root, PathRoot::Cache);
    assert_eq!(src.path, "manifests/sources/typescript/rules/src-001.md");
}

/// Test 9: duplicate id across overlays — same `UNI-001` declared
/// twice — fails with [`ResolveError::DuplicateRuleId`] regardless
/// of namespace ownership (which is `check::rules`'s problem).
#[test]
fn duplicate_rule_id_errors() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared universal",
    );
    write_rule(
        &project.path().join("adapters/targets/omnia/rules/uni-001-clone.md"),
        "UNI-001",
        "Clone in omnia overlay",
    );

    let sources = no_sources();
    let err =
        resolve(&inputs(project.path(), Some(rules_root.path()), "omnia", &sources)).unwrap_err();
    match err {
        ResolveError::DuplicateRuleId { id, paths } => {
            assert_eq!(id, "UNI-001");
            assert!(
                paths.contains("adapters/shared/rules/universal/uni-001.md"),
                "duplicate paths must cite the shared file: {paths}",
            );
            assert!(
                paths.contains("adapters/targets/omnia/rules/uni-001-clone.md"),
                "duplicate paths must cite the target overlay file: {paths}",
            );
        }
        other => panic!("expected DuplicateRuleId, got {other:?}"),
    }
}

/// Test 10: README.md (case-insensitive) is excluded from
/// discovery.
#[test]
fn readme_md_is_skipped() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared universal",
    );
    // README intentionally has no frontmatter — if discovery
    // walked into it the CH-11 parser would also raise a
    // ParseError, which the assertion below would catch.
    let readme = rules_root.path().join("adapters/shared/rules/universal/README.md");
    fs::write(&readme, "# Shared codex\n\nNotes about shared rules.\n").expect("write readme");

    let sources = no_sources();
    let result = resolve(&inputs(project.path(), Some(rules_root.path()), "omnia", &sources))
        .expect("resolve succeeds with README present");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].rule.id, "UNI-001");
}

/// Test 11: monorepo golden path. `project_dir == rules_root`
/// (probe step 2). Shared rules anchor on `RulesRoot`; project-local
/// target/source overlays anchor on `ProjectDir`, because they were
/// found via the project-local rung — not the rules-root fallback.
#[test]
fn monorepo_split_anchors() {
    let project = TempDir::new().expect("project");
    write_rule(
        &project.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared",
    );
    write_rule(
        &project.path().join("adapters/targets/omnia/rules/omnia-001.md"),
        "OMNIA-001",
        "Target",
    );
    write_rule(
        &project.path().join("adapters/sources/typescript/rules/src-001.md"),
        "SRC-001",
        "Source",
    );

    let sources = vec!["typescript".to_string()];
    let result = resolve(&inputs(project.path(), None, "omnia", &sources))
        .expect("resolve succeeds in monorepo layout");

    let shared = result.iter().find(|e| e.rule.id == "UNI-001").expect("shared present");
    let target = result.iter().find(|e| e.rule.id == "OMNIA-001").expect("target present");
    let source = result.iter().find(|e| e.rule.id == "SRC-001").expect("source present");

    assert_eq!(shared.path_root, PathRoot::RulesRoot);
    assert_eq!(target.path_root, PathRoot::ProjectDir);
    assert_eq!(source.path_root, PathRoot::ProjectDir);
}

/// In the monorepo probe-step-2 path, the rules-root fallback rung
/// must NOT also run for target/source overlays — otherwise a
/// project-local entry would shadow itself and surface as a
/// duplicate-id error. Regression guard for the explicit-vs-derived
/// rules-root distinction.
#[test]
fn monorepo_no_double_fallback_walk() {
    let project = TempDir::new().expect("project");
    write_rule(
        &project.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared",
    );
    write_rule(
        &project.path().join("adapters/targets/omnia/rules/omnia-001.md"),
        "OMNIA-001",
        "Target",
    );
    let sources = no_sources();
    let result = resolve(&inputs(project.path(), None, "omnia", &sources))
        .expect("resolve must not produce a duplicate-id error");
    assert_eq!(result.len(), 2);
}

/// Discovery is non-recursive — a stray nested rule must not be
/// picked up, mirroring CH-09's flat-directory expectation.
#[test]
fn discovery_is_non_recursive() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared",
    );
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/nested/uni-002.md"),
        "UNI-002",
        "Nested",
    );

    let sources = no_sources();
    let result = resolve(&inputs(project.path(), Some(rules_root.path()), "omnia", &sources))
        .expect("resolve succeeds");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].rule.id, "UNI-001");
}

/// Empty source adapter list and missing target overlay are not
/// errors — only shared rules surface.
#[test]
fn missing_overlays_are_silent() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared",
    );

    let sources: Vec<String> = vec!["unbound-source".to_string()];
    let result =
        resolve(&inputs(project.path(), Some(rules_root.path()), "unbound-target", &sources))
            .expect("missing overlays must not error");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].rule.id, "UNI-001");
}

/// Parse failures bubble up as [`ResolveError::Parse`] carrying
/// the offending absolute path.
#[test]
fn parse_error_includes_path() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    let bad_path = rules_root.path().join("adapters/shared/rules/universal/broken.md");
    fs::create_dir_all(bad_path.parent().unwrap()).expect("parent");
    fs::write(&bad_path, "no frontmatter here\n").expect("write broken rule");

    let sources = no_sources();
    let err =
        resolve(&inputs(project.path(), Some(rules_root.path()), "omnia", &sources)).unwrap_err();
    match err {
        ResolveError::Parse { path, .. } => {
            assert_eq!(path, bad_path);
        }
        other => panic!("expected Parse, got {other:?}"),
    }
}

/// `artifact_paths` / `languages` / `include_*` flags are accepted
/// but unused in CH-12 — the resolver must not error on populated
/// fields and must produce the same result as the default-empty
/// case so CH-13 can layer filtering on top without rewriting the
/// caller surface.
#[test]
fn ch13_inputs_are_accepted_but_ignored() {
    let rules_root = TempDir::new().expect("rules root");
    let project = TempDir::new().expect("project");
    write_rule(
        &rules_root.path().join("adapters/shared/rules/universal/uni-001.md"),
        "UNI-001",
        "Shared",
    );

    let artifact_paths = vec![PathBuf::from("crates/billing/src/lib.rs")];
    let languages = vec!["rust".to_string()];
    let sources = no_sources();
    let inputs = ResolveInputs {
        project_dir: project.path(),
        rules_root: Some(rules_root.path()),
        target_adapter: "omnia",
        source_adapters: &sources,
        artifact_paths: &artifact_paths,
        languages: &languages,
        include_deprecated: true,
        include_unmatched: true,
        include_core: true,
    };

    let result = resolve(&inputs).expect("resolve succeeds with CH-13 inputs populated");
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].rule.id, "UNI-001");
}

/// `rules-root-required` from CH-12 maps to a payload-free
/// `Error::Validation` so the wire envelope carries the closed
/// kebab discriminant in the top-level `error` code.
#[test]
fn maps_rules_root_required_to_validation() {
    let err = map_resolve_error(ResolveError::RulesRootRequired);
    match err {
        Error::Validation { code, .. } => {
            assert_eq!(code, "rules-root-required");
        }
        other => panic!("expected Error::Validation, got {other:?}"),
    }
}

/// `DuplicateRuleId` lands on a payload-free `Error::Validation`
/// keyed on `rules-duplicate-rule-id`, with the colliding id and
/// joined paths folded into the `detail` message.
#[test]
fn maps_duplicate_rule_id_to_validation() {
    let err = map_resolve_error(ResolveError::DuplicateRuleId {
        id: "UNI-001".into(),
        paths: "a.md, b.md".into(),
    });
    match err {
        Error::Validation { code, detail } => {
            assert_eq!(code, "rules-duplicate-rule-id");
            assert!(detail.contains("UNI-001"), "{detail}");
            assert!(detail.contains("a.md, b.md"), "{detail}");
        }
        other => panic!("expected Error::Validation, got {other:?}"),
    }
}

/// Filesystem failures map to `Error::Filesystem { op: "readdir" }`
/// so the JSON discriminant becomes `filesystem-readdir` (exit 1).
#[test]
fn maps_filesystem_to_filesystem_error() {
    let err = map_resolve_error(ResolveError::Filesystem {
        path: PathBuf::from("/missing"),
        source: io::Error::from(io::ErrorKind::NotFound),
    });
    match err {
        Error::Filesystem { op, path, .. } => {
            assert_eq!(op, "readdir");
            assert_eq!(path, PathBuf::from("/missing"));
        }
        other => panic!("expected Error::Filesystem, got {other:?}"),
    }
}

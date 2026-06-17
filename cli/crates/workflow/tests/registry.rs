//! Integration tests for `specify_workflow::registry::registry` and
//! `specify_workflow::registry::validate`.
//!
//! Deliberately narrow: the binary-level `tests/registry.rs` covers the
//! `registry {add,remove,validate}` wire surface (happy-path parses,
//! duplicate names, kebab violations, unknown top-level keys, the
//! workspace `url: .` rejection). This file keeps only what that layer
//! does not reach: serde parse edges, the URL-grammar accept/reject
//! tables, base-vs-workspace validation ordering, and the
//! contract-roles invariants (no binary test exercises `contracts:`).

use specify_error::Error;
use specify_workflow::registry::{ContractRoles, GreenfieldSeed, Registry, RegistryProject};
use tempfile::TempDir;

/// Scaffold `registry.yaml` (at the repo root) with `contents` and
/// return the containing project directory.
fn scaffold_registry(contents: &str) -> TempDir {
    let tmp = TempDir::new().unwrap();
    std::fs::write(Registry::path(tmp.path()), contents).unwrap();
    tmp
}

const MULTI_PROJECT_REGISTRY_YAML: &str = "\
version: 1
projects:
  - name: traffic
    url: .
    adapter: omnia@1.0.0
    description: Real-time traffic routing service
  - name: ingest
    url: git@github.com:augentic/ingest.git
    adapter: omnia@1.0.0
    description: Data ingestion pipeline
  - name: ops-runbook
    url: https://github.com/augentic/ops-runbook
    adapter: omnia@1.0.0
    description: Operational runbook reference
";

#[test]
fn registry_rejects_missing_version() {
    // Serde required-field error path — the binary tests only cover the
    // wrong-value case (`version: 2`), never an absent key.
    let yaml = "projects: []\n";
    let tmp = scaffold_registry(yaml);
    let err = Registry::load(tmp.path()).expect_err("missing version");
    match err {
        Error::Diag { detail: msg, .. } => assert!(msg.contains("version"), "msg: {msg}"),
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn registry_rejects_empty_string_name() {
    // The empty-name arm (`registry-project-name-empty`) is distinct
    // from the non-kebab arm the binary tests cover.
    let yaml = "\
version: 1
projects:
  - name: \"\"
    url: .
    adapter: omnia@1.0.0
";
    let tmp = scaffold_registry(yaml);
    let err = Registry::load(tmp.path()).expect_err("empty name");
    match err {
        Error::Diag { detail: msg, .. } => {
            assert!(msg.contains("empty") || msg.contains("kebab-case"), "msg: {msg}");
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn registry_accepts_empty_string_adapter() {
    // An empty `adapter` seed is harmless — the registry does not
    // author the project's target adapter, which lives in its own
    // `project.yaml`.
    let yaml = "\
version: 1
projects:
  - name: traffic
    url: .
    adapter: \"\"
";
    let tmp = scaffold_registry(yaml);
    let registry = Registry::load(tmp.path()).expect("parses").expect("present");
    assert_eq!(registry.projects[0].adapter.as_deref(), Some(""));
}

#[test]
fn accepts_multi_project_not_single_repo() {
    // Pins the false branch of `is_single_repo`; the binary's
    // `load_from_tempdir` only asserts the single-project true branch.
    let tmp = scaffold_registry(MULTI_PROJECT_REGISTRY_YAML);
    let registry = Registry::load(tmp.path()).unwrap().unwrap();
    assert_eq!(registry.projects.len(), 3);
    assert!(!registry.is_single_repo());
}

// ---------- Registry URL validation (registry URL validation) ----------

fn registry_with_one_url(url: &str) -> Registry {
    Registry {
        version: 1,
        projects: vec![RegistryProject {
            name: "traffic".into(),
            url: url.into(),
            adapter: Some("omnia@1.0.0".into()),
            description: None,
            contracts: None,
            greenfield_seed: None,
        }],
    }
}

#[test]
fn project_url_materialises_as_symlink() {
    for (url, symlink) in [
        (".", true),
        ("../peer", true),
        ("./foo", true),
        ("pkg/sub", true),
        ("git@github.com:augentic/ingest.git", false),
        ("https://github.com/augentic/ops-runbook", false),
        ("http://example.com/repo.git", false),
        ("ssh://git@github.com/augentic/specify.git", false),
        ("git+https://example.com/org/repo.git", false),
        ("git+http://example.com/org/repo.git", false),
        ("git+ssh://git@github.com/org/repo.git", false),
    ] {
        let p = RegistryProject {
            name: "traffic".into(),
            url: url.into(),
            adapter: Some("omnia@1.0.0".into()),
            description: None,
            contracts: None,
            greenfield_seed: None,
        };
        assert_eq!(p.is_local(), symlink, "url={url:?}");
    }
}

#[test]
fn greenfield_seed_accepts_kebab_domains() {
    let mut registry = registry_with_one_url(".");
    registry.projects[0].greenfield_seed = Some(GreenfieldSeed {
        domains: vec!["identity".into(), "account-creation".into()],
    });
    registry.validate_shape().expect("kebab seed domains validate");
}

#[test]
fn greenfield_seed_rejects_non_kebab_domain() {
    let mut registry = registry_with_one_url(".");
    registry.projects[0].greenfield_seed = Some(GreenfieldSeed {
        domains: vec!["Identity_Domain".into()],
    });
    let err = registry.validate_shape().expect_err("non-kebab seed domain is rejected");
    match err {
        Error::Diag { code, .. } => {
            assert_eq!(code, "registry-greenfield-seed-domain-not-kebab");
        }
        other => panic!("expected a Diag error, got: {other}"),
    }
}

#[test]
fn registry_accepts_url_shapes_for_c28() {
    for url in [
        "https://github.com/a/b",
        "http://github.com/a/b",
        "git@github.com:org/repo.git",
        "ssh://git@github.com/org/repo.git",
        "git+https://github.com/org/repo.git",
        "../peer-repo",
        "./inputs/legacy",
        "inputs/runbook",
    ] {
        registry_with_one_url(url).validate_shape().unwrap_or_else(|e| {
            panic!("expected url {url:?} to validate, got: {e}");
        });
    }
}

#[test]
fn rejects_malformed_url_shapes() {
    // Each row hits a distinct rejection arm in `validate_project_url`
    // (plus the empty-string arm in `validate_shape`). The binary's
    // `registry validate` tests only exercise well-formed URLs, so the
    // rejection grammar is pinned here.
    for (url, fragment) in [
        ("", "url"),                              // registry-project-url-empty
        ("   ", "whitespace"),                    // whitespace-only url
        (" https://example.com/a", "whitespace"), // leading whitespace
        ("ftp://example.com/repo", "scheme"),     // unsupported scheme
        ("file:///tmp/repo", "scheme"),           // file:// is not a remote
        ("weird:path", ":"),                      // colon without scheme or git@
        ("/absolute/path", "relative"),           // absolute filesystem path
    ] {
        let err = registry_with_one_url(url)
            .validate_shape()
            .expect_err(&format!("expected url {url:?} to be rejected"));
        match err {
            Error::Diag { detail: msg, .. } => {
                assert!(msg.contains(fragment), "url {url:?}: msg: {msg}");
            }
            other => panic!("wrong variant for url {url:?}: {other:?}"),
        }
    }
}

// ---------- Registry workspace validation (registry workspace validation) ----------

#[test]
fn workspace_inherits_base_errors() {
    // version != 1 is a base-shape error; workspace mode must surface it
    // without ever reaching the `workspace-cannot-be-project` check.
    let reg = Registry {
        version: 2,
        projects: vec![],
    };
    let err = reg.validate_shape_workspace().expect_err("base shape error must propagate through");
    match err {
        Error::Diag { code, detail: msg } => {
            assert!(msg.contains("version"), "msg: {msg}");
            assert_ne!(
                code, "workspace-cannot-be-project",
                "must not short-circuit base-shape errors with the workspace diagnostic: {msg}"
            );
        }
        other => panic!("wrong error variant: {other:?}"),
    }
}

// ---------- Registry contract roles (registry-layer invariants) ----------

const REGISTRY_WITH_CONTRACT_ROLES_YAML: &str = "\
version: 1
projects:
  - name: traffic
    url: .
    adapter: omnia@1.0.0
    description: Real-time traffic routing service
    contracts:
      produces:
        - http/traffic-api.yaml
      consumes:
        - http/ingest-api.yaml
  - name: ingest
    url: git@github.com:augentic/ingest.git
    adapter: omnia@1.0.0
    description: Data ingestion pipeline
    contracts:
      produces:
        - http/ingest-api.yaml
      consumes:
        - schemas/order-placed.yaml
";

#[test]
fn with_contract_roles_parses_and_validates() {
    let tmp = scaffold_registry(REGISTRY_WITH_CONTRACT_ROLES_YAML);
    let registry = Registry::load(tmp.path()).expect("parses").expect("present");
    assert_eq!(registry.projects.len(), 2);

    let traffic = &registry.projects[0];
    let roles = traffic.contracts.as_ref().expect("traffic has contracts");
    assert_eq!(roles.produces, vec!["http/traffic-api.yaml"]);
    assert_eq!(roles.consumes, vec!["http/ingest-api.yaml"]);

    let ingest = &registry.projects[1];
    let roles = ingest.contracts.as_ref().expect("ingest has contracts");
    assert_eq!(roles.produces, vec!["http/ingest-api.yaml"]);
    assert_eq!(roles.consumes, vec!["schemas/order-placed.yaml"]);
}

#[test]
fn contract_roles_round_trip_omits_empty() {
    let original = Registry {
        version: 1,
        projects: vec![RegistryProject {
            name: "traffic".into(),
            url: ".".into(),
            adapter: Some("omnia@1.0.0".into()),
            description: None,
            contracts: Some(ContractRoles {
                produces: vec!["http/traffic-api.yaml".into()],
                consumes: vec![],
            }),
            greenfield_seed: None,
        }],
    };
    let yaml = serde_saphyr::to_string(&original).expect("serialize");
    assert!(!yaml.contains("consumes"), "empty consumes should be omitted: {yaml}");
    let round_tripped: Registry = serde_saphyr::from_str(&yaml).expect("re-parse");
    assert_eq!(round_tripped, original);
}

#[test]
fn rejects_single_producer_violation() {
    let yaml = "\
version: 1
projects:
  - name: alpha
    url: .
    adapter: omnia@1.0.0
    description: Alpha service
    contracts:
      produces:
        - http/shared-api.yaml
  - name: beta
    url: ../beta
    adapter: omnia@1.0.0
    description: Beta service
    contracts:
      produces:
        - http/shared-api.yaml
";
    let tmp = scaffold_registry(yaml);
    let err = Registry::load(tmp.path()).expect_err("single producer violation");
    match err {
        Error::Diag { detail: msg, .. } => {
            assert!(msg.contains("http/shared-api.yaml"), "msg: {msg}");
            assert!(msg.contains("produced by both"), "msg: {msg}");
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

/// registry contract cleanup dropped `contracts.imports`. Any registry that still
/// declares the field after the upgrade fails fast at parse time
/// (`#[serde(deny_unknown_fields)]`) — that diagnostic is the
/// documented migration trigger from registry contract cleanup §Migration.
#[test]
fn registry_rejects_unknown_imports_field() {
    let yaml = "\
version: 1
projects:
  - name: alpha
    url: .
    adapter: omnia@1.0.0
    contracts:
      imports:
        - http/external-api.yaml
";
    let tmp = scaffold_registry(yaml);
    let err = Registry::load(tmp.path()).expect_err("legacy imports field rejected");
    match err {
        Error::Diag { detail: msg, .. } => assert!(msg.contains("imports"), "msg: {msg}"),
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn rejects_absolute_path_in_contract_role() {
    let yaml = "\
version: 1
projects:
  - name: alpha
    url: .
    adapter: omnia@1.0.0
    contracts:
      produces:
        - /absolute/path.yaml
";
    let tmp = scaffold_registry(yaml);
    let err = Registry::load(tmp.path()).expect_err("absolute path rejected");
    match err {
        Error::Diag { detail: msg, .. } => {
            assert!(msg.contains("/absolute/path.yaml"), "msg: {msg}");
            assert!(msg.contains("relative"), "msg: {msg}");
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

#[test]
fn rejects_self_consistency_violation() {
    let yaml = "\
version: 1
projects:
  - name: alpha
    url: .
    adapter: omnia@1.0.0
    contracts:
      produces:
        - http/my-api.yaml
      consumes:
        - http/my-api.yaml
";
    let tmp = scaffold_registry(yaml);
    let err = Registry::load(tmp.path()).expect_err("self-consistency violation");
    match err {
        Error::Diag { detail: msg, .. } => {
            assert!(msg.contains("alpha"), "msg: {msg}");
            assert!(msg.contains("http/my-api.yaml"), "msg: {msg}");
            assert!(msg.contains("produces"), "msg: {msg}");
            assert!(msg.contains("consumes"), "msg: {msg}");
        }
        other => panic!("wrong variant: {other:?}"),
    }
}

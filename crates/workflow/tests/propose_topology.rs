//! Integration coverage for the greenfield-seed projection
//! (`workflow::change::apply_greenfield_seed`): seed → empty
//! greenfield surface, seed shadowed by a baseline, absent seed no-op.

use workflow::change::{ProjectRef, apply_greenfield_seed};
use workflow::registry::catalog::{GreenfieldSeed, Registry, RegistryProject};
use workflow::registry::topology::Surface;

fn project_ref(name: &str, surface: Vec<Surface>) -> ProjectRef {
    ProjectRef {
        name: name.to_string(),
        target: "demo-target@1.0.0".to_string(),
        description: None,
        surface,
        recent: Vec::new(),
        decisions: Vec::new(),
        decisions_more: None,
        platforms: Vec::new(),
    }
}

fn registry_with_seed(name: &str, domains: &[&str]) -> Registry {
    Registry {
        version: 1,
        projects: vec![RegistryProject {
            name: name.to_string(),
            url: ".".to_string(),
            adapter: Some("demo-target@1.0.0".to_string()),
            description: None,
            contracts: None,
            greenfield_seed: Some(GreenfieldSeed {
                domains: domains.iter().map(|d| (*d).to_string()).collect(),
            }),
        }],
    }
}

#[test]
fn seed_projects_into_empty_surface() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut topology = vec![project_ref("svc", Vec::new())];
    let registry = registry_with_seed("svc", &["identity", "billing"]);

    let findings = apply_greenfield_seed(&mut topology, &registry, dir.path(), false);

    assert!(findings.is_empty(), "no baseline means no shadow finding");
    let domains: Vec<&str> = topology[0].surface.iter().map(|s| s.domain.as_str()).collect();
    assert_eq!(domains, ["identity", "billing"]);
    assert!(
        topology[0].surface.iter().all(|s| s.requirements.is_empty()),
        "seeded domains carry empty requirements"
    );
}

#[test]
fn seed_is_shadowed_once_baseline_exists() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join(".specify/specs/identity")).expect("specs");
    let existing = vec![Surface {
        domain: "identity".to_string(),
        requirements: vec!["Sign in".to_string()],
        more: None,
    }];
    let mut topology = vec![project_ref("svc", existing.clone())];
    let registry = registry_with_seed("svc", &["billing"]);

    let findings = apply_greenfield_seed(&mut topology, &registry, dir.path(), false);

    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id.as_deref(), Some("greenfield-seed-shadowed"));
    assert_eq!(topology[0].surface, existing, "real surface supersedes the seed");
}

#[test]
fn absent_seed_is_a_noop() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut topology = vec![project_ref("svc", Vec::new())];
    let registry = Registry {
        version: 1,
        projects: Vec::new(),
    };

    let findings = apply_greenfield_seed(&mut topology, &registry, dir.path(), false);

    assert!(findings.is_empty());
    assert!(topology[0].surface.is_empty());
}

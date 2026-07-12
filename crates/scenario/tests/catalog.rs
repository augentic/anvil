//! Canonical catalog validation against checked-in workflow scenarios.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use scenario::{Scenario, assertion_registry};

#[test]
fn every_scenario_loads() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../quality/scenarios");
    let mut paths = fs::read_dir(&root)
        .expect("quality/scenarios exists")
        .map(|entry| entry.expect("scenario entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "yaml"))
        .collect::<Vec<_>>();
    paths.sort();
    assert!(!paths.is_empty(), "canonical scenario catalog is non-empty");

    for path in paths {
        Scenario::load(&path).unwrap_or_else(|error| {
            panic!("{} is a valid canonical scenario: {error}", path.display())
        });
    }
}

#[test]
fn runbooks_match_canonical_catalog() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let runbooks = fs::read_dir(repo.join("quality/runbooks"))
        .expect("quality runbooks")
        .map(|entry| entry.expect("runbook entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "md"))
        .filter_map(|path| {
            let stem = path.file_stem()?.to_str()?;
            (stem != "README").then(|| stem.to_string())
        })
        .collect::<BTreeSet<_>>();
    let canonical = fs::read_dir(repo.join("quality/scenarios"))
        .expect("canonical scenarios")
        .map(|entry| entry.expect("canonical entry").path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "yaml"))
        .map(|path| Scenario::load(&path).expect("canonical scenario"))
        .filter(|scenario| scenario.owner == "scenarios")
        .map(|scenario| scenario.id)
        .collect::<BTreeSet<_>>();

    assert_eq!(canonical, runbooks);
}

#[test]
fn assertion_documentation_matches_registry() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let document = fs::read_to_string(repo.join("quality/reference/assertions.md"))
        .expect("assertion document");
    let documented = document
        .lines()
        .filter_map(|line| line.strip_prefix("### `")?.strip_suffix('`'))
        .map(str::to_string)
        .collect::<BTreeSet<_>>();
    let registered = assertion_registry()
        .iter()
        .map(|metadata| metadata.id.to_string())
        .collect::<BTreeSet<_>>();

    assert_eq!(documented, registered);
}

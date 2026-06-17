use std::io;
use std::process::{Command, ExitStatus, Output};

use super::*;

/// `CmdRunner` stub for `git rev-parse HEAD`: `Some(sha)` resolves to a
/// successful run printing that sha; `None` is a failed git invocation
/// (no checkout / unresolvable ref), so [`resolve_head`] yields `None`.
fn fake_git(head: Option<&str>) -> impl Fn(&mut Command) -> io::Result<Output> {
    let head = head.map(str::to_string);
    move |_cmd: &mut Command| {
        Ok(head.as_ref().map_or_else(
            || Output {
                status: exit_status(1 << 8),
                stdout: Vec::new(),
                stderr: b"not a git repository".to_vec(),
            },
            |sha| Output {
                status: exit_status(0),
                stdout: format!("{sha}\n").into_bytes(),
                stderr: Vec::new(),
            },
        ))
    }
}

#[cfg(unix)]
fn exit_status(raw: i32) -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(raw)
}

#[test]
fn classify_matches_ok() {
    assert_eq!(classify_status(Some("abc"), Some("abc")), PluginStatus::Ok);
}

#[test]
fn classify_differs_drifted() {
    assert_eq!(classify_status(Some("abc"), Some("def")), PluginStatus::Drifted);
}

#[test]
fn classify_unresolvable_present() {
    assert_eq!(classify_status(Some("abc"), None), PluginStatus::Present);
}

#[test]
fn classify_no_cache_missing() {
    assert_eq!(classify_status(None, Some("abc")), PluginStatus::Missing);
    assert_eq!(classify_status(None, None), PluginStatus::Missing);
}

/// Build a `<name>/<plugin>/<sha>/` cache tree plus a sibling
/// marketplace.json under a tempdir, returning the marketplace path.
fn fixture(tmp: &Path, name: &str, leaves: &[(&str, Option<&str>)]) -> (PathBuf, PathBuf) {
    let root = cache_root(tmp, name);
    for (plugin, sha) in leaves {
        let plugin_dir = root.join(plugin);
        fs::create_dir_all(&plugin_dir).unwrap();
        if let Some(sha) = sha {
            fs::create_dir_all(plugin_dir.join(sha)).unwrap();
        }
    }
    let marketplace = tmp.join("marketplace.json");
    fs::write(&marketplace, "{}").unwrap();
    (marketplace, root)
}

fn manifest(name: &str, plugins: &[(&str, &str)]) -> MarketplaceManifest {
    MarketplaceManifest {
        name: name.to_string(),
        plugin_root: "plugins".to_string(),
        plugins: plugins
            .iter()
            .map(|(n, s)| PluginEntry {
                name: (*n).to_string(),
                source: (*s).to_string(),
                description: None,
            })
            .collect(),
    }
}

#[test]
fn report_flags_missing_extra_and_present() {
    let tmp = tempfile::tempdir().unwrap();
    // `spec` declared with a cache leaf; `omnia` is an undeclared
    // extra; `client` is declared with no cache leaf.
    let (marketplace, root) =
        fixture(tmp.path(), "augentic", &[("spec", Some("cafe")), ("omnia", Some("beef"))]);
    let mani = manifest("augentic", &[("spec", "spec"), ("client", "client")]);
    // Expected unresolvable -> declared+cached collapses to present.
    let runner = fake_git(None);

    let report = build_report(&marketplace, &mani, &root, &runner).unwrap();

    let by_name = |n: &str| report.plugins.iter().find(|p| p.name == n).unwrap().status;
    assert_eq!(by_name("spec"), PluginStatus::Present, "cached but expected unresolvable");
    assert_eq!(by_name("client"), PluginStatus::Missing, "declared, no cache leaf");
    assert_eq!(by_name("omnia"), PluginStatus::Extra, "cached, not declared");
    assert_eq!(report.summary.present, 1);
    assert_eq!(report.summary.missing, 1);
    assert_eq!(report.summary.extra, 1);
}

#[test]
fn report_drifted_and_ok_with_resolved_head() {
    let tmp = tempfile::tempdir().unwrap();
    let (marketplace, root) =
        fixture(tmp.path(), "augentic", &[("spec", Some("oldsha")), ("client", Some("head"))]);
    let mani = manifest("augentic", &[("spec", "spec"), ("client", "client")]);
    // Relative-path sources share the resolved HEAD.
    let runner = fake_git(Some("head"));

    let report = build_report(&marketplace, &mani, &root, &runner).unwrap();

    let by_name = |n: &str| report.plugins.iter().find(|p| p.name == n).unwrap();
    assert_eq!(by_name("spec").status, PluginStatus::Drifted);
    assert_eq!(by_name("spec").expected_sha.as_deref(), Some("head"));
    assert_eq!(by_name("client").status, PluginStatus::Ok);
    assert_eq!(report.summary.ok, 1);
    assert_eq!(report.summary.drifted, 1);
}

#[test]
fn report_missing_cache_root_is_all_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let marketplace = tmp.path().join("marketplace.json");
    fs::write(&marketplace, "{}").unwrap();
    let root = cache_root(tmp.path(), "augentic");
    let mani = manifest("augentic", &[("spec", "spec")]);
    let runner = fake_git(Some("head"));

    let report = build_report(&marketplace, &mani, &root, &runner).unwrap();
    assert_eq!(report.plugins[0].status, PluginStatus::Missing);
    assert_eq!(report.summary.missing, 1);
}

#[test]
fn refresh_deletes_only_scoped_root() {
    let tmp = tempfile::tempdir().unwrap();
    let (marketplace, root) = fixture(tmp.path(), "augentic", &[("spec", Some("cafe"))]);
    let (_, other) = fixture(tmp.path(), "acme", &[("widget", Some("beef"))]);

    let outcome = refresh(&marketplace, &root).unwrap();
    assert_eq!(outcome.deleted_paths.len(), 1);
    assert!(!root.exists(), "scoped cache removed");
    assert!(other.exists(), "sibling marketplace cache survives");
}

#[test]
fn refresh_missing_root_is_noop() {
    let tmp = tempfile::tempdir().unwrap();
    let marketplace = tmp.path().join("marketplace.json");
    let root = cache_root(tmp.path(), "augentic");
    let outcome = refresh(&marketplace, &root).unwrap();
    assert!(outcome.deleted_paths.is_empty());
}

#[test]
fn load_marketplace_parses_and_validates() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("marketplace.json");
    fs::write(
        &path,
        r#"{
          "name": "augentic",
          "owner": { "name": "augentic", "email": "info@augentic.io" },
          "metadata": { "description": "d", "version": "0.27.0", "pluginRoot": "plugins" },
          "plugins": [ { "name": "spec", "source": "spec", "description": "Spec skills." } ]
        }"#,
    )
    .unwrap();
    let manifest = load_marketplace(&path).unwrap();
    assert_eq!(manifest.name, "augentic");
    assert_eq!(manifest.plugin_root, "plugins");
    assert_eq!(manifest.plugins[0].source, "spec");
}

#[test]
fn marketplace_rejects_schema_violation() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("marketplace.json");
    fs::write(&path, r#"{ "name": "augentic" }"#).unwrap();
    let err = load_marketplace(&path).expect_err("missing required fields");
    assert_eq!(err.variant_str(), "marketplace-schema");
}

#[test]
fn discover_prefers_flag_then_project() {
    let tmp = tempfile::tempdir().unwrap();
    let project = tmp.path().join("proj");
    let cursor_plugin = project.join(".cursor-plugin");
    fs::create_dir_all(&cursor_plugin).unwrap();
    let project_file = cursor_plugin.join("marketplace.json");
    fs::write(&project_file, "{}").unwrap();

    // No flag -> project hit.
    let found = discover_marketplace(None, &project).unwrap();
    assert_eq!(found, project_file);

    // Flag overrides.
    let flag = tmp.path().join("custom.json");
    fs::write(&flag, "{}").unwrap();
    let found = discover_marketplace(Some(&flag), &project).unwrap();
    assert_eq!(found, flag);
}

#[test]
fn discover_missing_flag_errors() {
    let tmp = tempfile::tempdir().unwrap();
    let err = discover_marketplace(Some(&tmp.path().join("nope.json")), tmp.path())
        .expect_err("missing flag path");
    assert_eq!(err.variant_str(), "marketplace-flag-missing");
}

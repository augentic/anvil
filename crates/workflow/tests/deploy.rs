//! Integration tests for the deployment-manifest generator
//! (`workflow::deploy`).
//!
//! Covers the generated document shape (guests, mount, routes,
//! transport, the core link allow-list — asserted on parsed TOML, not
//! byte goldens), the load-bearing presence gate (a dangling pinned
//! entry is `adapter-not-installed` naming the identity and the sync
//! command; a dangling bare component is `adapter-not-found`), and
//! path escaping for hostile mount paths.

use std::fs;
use std::path::{Path, PathBuf};

use error::Error;
use workflow::adapter::Axis;
use workflow::deploy::{DeployGuest, generate, manifest_path};

use crate::common;

fn component(dir: &Path, name: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, b"\0asm-component").expect("write component");
    path
}

fn pinned_guest(axis: Axis, name: &str, version: &str, component: PathBuf) -> DeployGuest {
    DeployGuest {
        axis,
        name: name.to_string(),
        version: Some(semver::Version::parse(version).expect("semver")),
        component,
    }
}

fn bare_guest(axis: Axis, name: &str, component: PathBuf) -> DeployGuest {
    DeployGuest {
        axis,
        name: name.to_string(),
        version: None,
        component,
    }
}

#[test]
fn generates_manifest_into_project_cache() {
    // The generated manifest lands in the per-project deployment
    // tenant and carries the full shape the composed runtime reads:
    // the core guest with the adapter-contract link allow-list, one
    // `[[guest]]` + `/mcp/<name>` route per adapter (pinned store
    // entries and project-local bare names alike), the writable `"."`
    // mount over the project dir, and the in-process transport.
    let tmp = tempfile::tempdir().expect("tempdir");
    let _cache = common::scoped_cache(tmp.path());
    let project = tmp.path().join("project");
    fs::create_dir_all(&project).expect("project dir");
    let core = component(tmp.path(), "specify.wasm");
    let omnia = component(tmp.path(), "omnia@1.0.0.wasm");
    let intent = component(tmp.path(), "intent.wasm");
    let guests = [
        pinned_guest(Axis::Target, "omnia", "1.0.0", omnia.clone()),
        bare_guest(Axis::Source, "intent", intent.clone()),
    ];

    let path = generate(&project, &core, &guests).expect("generate manifest");
    assert_eq!(path, manifest_path(&project), "written at the deployment-tenant path");
    assert_eq!(
        path.parent().and_then(Path::parent),
        Some(common::expected_cache_dir(&project).as_path()),
        "the deployment tenant lives in the per-project cache"
    );

    let doc: toml::Value =
        toml::from_str(&fs::read_to_string(&path).expect("manifest contents")).expect("valid TOML");
    let guests = doc["guest"].as_array().expect("guest array");
    let ids: Vec<&str> = guests.iter().map(|g| g["id"].as_str().expect("id")).collect();
    assert_eq!(ids, ["workflow", "target:omnia", "source:intent"]);
    assert_eq!(guests[0]["source"]["path"].as_str(), Some(core.display().to_string().as_str()));
    assert_eq!(
        guests[0]["link"].as_array().map(Vec::len),
        Some(2),
        "the core guest links both adapter-contract interfaces"
    );
    assert_eq!(guests[1]["source"]["path"].as_str(), Some(omnia.display().to_string().as_str()));
    assert_eq!(guests[2]["source"]["path"].as_str(), Some(intent.display().to_string().as_str()));

    let mounts = doc["mount"].as_array().expect("mount array");
    assert_eq!(
        mounts.len(),
        3,
        "the project mount, the derived-cache mount, and the read-only store mount"
    );
    assert_eq!(mounts[0]["name"].as_str(), Some("."));
    assert_eq!(mounts[0]["path"].as_str(), Some(project.display().to_string().as_str()));
    assert_eq!(mounts[0]["writable"].as_bool(), Some(true));
    // guest routing: the per-project derived cache is mounted so the
    // guest's scaffold leg reaches the cache tenants.
    assert_eq!(mounts[1]["name"].as_str(), Some(schema::cache::GUEST_CACHE_MOUNT));
    assert_eq!(
        mounts[1]["path"].as_str(),
        Some(schema::cache::project_cache_dir(&project).display().to_string().as_str())
    );
    assert_eq!(mounts[1]["writable"].as_bool(), Some(true));
    // global store mount: the global adapter store is mounted read-only so
    // forwarded verbs resolve pinned identities in-guest (hydration
    // stays native).
    assert_eq!(mounts[2]["name"].as_str(), Some(schema::cache::GUEST_STORE_MOUNT));
    assert_eq!(
        mounts[2]["path"].as_str(),
        Some(schema::cache::adapter_store_root().display().to_string().as_str())
    );
    assert_eq!(mounts[2]["writable"].as_bool(), Some(false));

    let routes = doc["route"]["http"].as_array().expect("http routes");
    let prefixes: Vec<(&str, &str)> = routes
        .iter()
        .map(|r| (r["prefix"].as_str().expect("prefix"), r["guest"].as_str().expect("guest")))
        .collect();
    assert_eq!(prefixes, [("/mcp/omnia", "target:omnia"), ("/mcp/intent", "source:intent")]);
    assert_eq!(doc["transport"]["default"].as_str(), Some("in-process"));
}

#[test]
fn dangling_pinned_entry_is_not_installed() {
    // The store is load-bearing: a pinned component missing at
    // generation time aborts with the typed `adapter-not-installed`
    // (naming the identity and the literal sync command) and writes no
    // manifest — never a generated-then-broken deployment.
    let tmp = tempfile::tempdir().expect("tempdir");
    let _cache = common::scoped_cache(tmp.path());
    let core = component(tmp.path(), "specify.wasm");
    let guests =
        [pinned_guest(Axis::Target, "omnia", "1.0.0", tmp.path().join("omnia@1.0.0.wasm"))];

    let err = generate(tmp.path(), &core, &guests).expect_err("dangling pinned entry must fail");
    let detail = err.to_string();
    assert!(
        matches!(&err, Error::Validation { code, .. } if *code == "adapter-not-installed"),
        "{detail}"
    );
    assert!(detail.contains("omnia@1.0.0"), "error names the identity: {detail}");
    assert!(detail.contains("specify adapters sync"), "error names the sync command: {detail}");
    assert!(!manifest_path(tmp.path()).exists(), "no manifest is written on a presence failure");
}

#[test]
fn dangling_bare_component_is_not_found() {
    // A project-local bare-name component that vanished resolves to
    // the resolver's own `adapter-not-found` vocabulary — sync cannot
    // remedy a development artifact.
    let tmp = tempfile::tempdir().expect("tempdir");
    let _cache = common::scoped_cache(tmp.path());
    let core = component(tmp.path(), "specify.wasm");
    let guests = [bare_guest(Axis::Source, "intent", tmp.path().join("intent.wasm"))];

    let err = generate(tmp.path(), &core, &guests).expect_err("dangling bare component must fail");
    let detail = err.to_string();
    assert!(
        matches!(
            err,
            Error::Diag {
                code: "adapter-not-found",
                ..
            }
        ),
        "{detail}"
    );
    assert!(detail.contains("`intent`"), "error names the adapter: {detail}");
}

#[test]
fn hostile_paths_stay_parseable() {
    // Host paths are emitted as escaped TOML strings, so quotes and
    // backslashes in the mount path survive the round-trip.
    let tmp = tempfile::tempdir().expect("tempdir");
    let _cache = common::scoped_cache(tmp.path());
    let project = tmp.path().join("we\"ird\\dir");
    fs::create_dir_all(&project).expect("hostile project dir");
    let core = component(tmp.path(), "specify.wasm");

    let path = generate(&project, &core, &[]).expect("generate manifest");
    let doc: toml::Value =
        toml::from_str(&fs::read_to_string(&path).expect("manifest contents")).expect("valid TOML");
    assert_eq!(
        doc["mount"].as_array().expect("mount array")[0]["path"].as_str(),
        Some(project.display().to_string().as_str()),
        "the hostile mount path round-trips through the TOML string escape"
    );

    // Control characters (TOML basic strings forbid raw U+0000–U+001F
    // and U+007F) escape to \n / \t / \r / \u00XX and round-trip. The
    // mount path never has to exist on disk, so the scenario stays
    // portable to filesystems that refuse control chars in names.
    let control = tmp.path().join("ctrl\nnew\tline\r\u{1}\u{7f}dir");
    let path = generate(&control, &core, &[]).expect("generate manifest with control chars");
    let doc: toml::Value =
        toml::from_str(&fs::read_to_string(&path).expect("manifest contents")).expect("valid TOML");
    assert_eq!(
        doc["mount"].as_array().expect("mount array")[0]["path"].as_str(),
        Some(control.display().to_string().as_str()),
        "the control-char mount path round-trips through the TOML string escape"
    );
}

//! Integration tests for `specify adapters sync` — the explicit
//! RFC-65 hydration trigger.
//!
//! Warm-store probes only: the networked fetch leg (a registry pull on
//! a store miss) is exercised by the `#[ignore]` registry smoke tests;
//! everything here stages verified store entries via
//! `stage_store_component` and asserts the resolved-set envelope, the
//! `--frozen` refusal, the `.specify/adapters.lock` append, the
//! `adapters.synced` journal event, and the regenerated deployment
//! manifest in the per-project cache (RFC-65).

use std::fs;
use std::path::Path;

use tempfile::tempdir;

use crate::common::{
    self, parse_json, read_journal_normalized, specify_cmd, stage_store_component,
};

/// Seed a minimal initialised project whose `project.yaml` declares
/// the given `adapters:` prefetch entries.
fn seed_project_with_prefetch(root: &Path, entries: &[&str]) {
    let specify = root.join(".specify");
    fs::create_dir_all(&specify).expect("mkdir .specify");
    let mut body = "name: demo\nadapter: omnia\nspecify: 0.1.0\n".to_string();
    if !entries.is_empty() {
        body.push_str("adapters:\n");
        for entry in entries {
            body.push_str("- ");
            body.push_str(entry);
            body.push('\n');
        }
    }
    fs::write(specify.join("project.yaml"), body).expect("write project.yaml");
}

#[test]
fn warm_store_resolves_and_prints_set() {
    // A warm store makes sync a no-op probe: the staged entry
    // satisfies the pin without any fetch, the JSON envelope carries
    // the resolved row (identity, store path, digest) plus the
    // counts, and the entry survives byte-identical.
    let tmp = tempdir().unwrap();
    let entry = stage_store_component("demo-target", "1.0.0");
    let bytes_before = fs::read(&entry).expect("read staged entry");
    seed_project_with_prefetch(tmp.path(), &["demo-target@1.0.0"]);

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapters", "sync"])
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["resolved"], 1);
    assert_eq!(body["fetched"], 0);
    assert_eq!(body["already-present"], 1);
    assert_eq!(body["frozen"], false);
    assert_eq!(body["adapters"][0]["name"], "demo-target");
    assert_eq!(body["adapters"][0]["version"], "1.0.0");
    assert_eq!(body["adapters"][0]["path"], entry.display().to_string());
    assert!(
        body["adapters"][0]["digest"].as_str().is_some_and(|d| d.starts_with("sha256:")),
        "digest is the sha256-prefixed content digest: {body}"
    );
    assert_eq!(
        fs::read(&entry).expect("re-read staged entry"),
        bytes_before,
        "a warm-store sync must leave the entry untouched"
    );
}

#[test]
fn warm_store_text_says_noop() {
    // The text renderer names the no-op probe when nothing was
    // fetched, and still prints one row per resolved identity.
    let tmp = tempdir().unwrap();
    stage_store_component("demo-target", "1.0.0");
    seed_project_with_prefetch(tmp.path(), &["demo-target@1.0.0"]);

    let assert =
        specify_cmd().current_dir(tmp.path()).args(["adapters", "sync"]).assert().success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("no-op probe"), "warm-store sync says so: {stdout}");
    assert!(stdout.contains("demo-target@1.0.0"), "row names the identity: {stdout}");
}

#[test]
fn sync_collects_plan_source_pins() {
    // The sync trigger hydrates the *full* declared set: `plan.yaml`
    // source pins join the `project.yaml` prefetch list through the
    // shared `hydrate::collect_refs` path. The plan-bound pin stages
    // source-axis bytes — manifest regeneration resolves the binding
    // through the source resolver, which gates on the exported axis.
    let tmp = tempdir().unwrap();
    stage_store_component("demo-target", "1.0.0");
    stage_store_source_component("demo-source", "2.0.0");
    seed_project_with_prefetch(tmp.path(), &["demo-target@1.0.0"]);
    fs::write(
        tmp.path().join("plan.yaml"),
        "name: demo\n\
         sources:\n\
         \x20 ts:\n\
         \x20   adapter: demo-source\n\
         \x20   version: 2.0.0\n\
         \x20   path: ./src\n\
         slices: []\n",
    )
    .expect("write plan.yaml");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapters", "sync"])
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["resolved"], 2);
    let names: Vec<&str> = body["adapters"]
        .as_array()
        .expect("adapters array")
        .iter()
        .map(|row| row["name"].as_str().expect("name"))
        .collect();
    assert_eq!(names, vec!["demo-target", "demo-source"], "prefetch pin then plan pin");
}

#[test]
fn sync_appends_adapters_lock_and_journals() {
    // A successful sync pins each new identity's digest into the
    // committed `.specify/adapters.lock` and appends one
    // `adapters.synced` journal event carrying the counts.
    let tmp = tempdir().unwrap();
    let entry = stage_store_component("demo-target", "1.0.0");
    seed_project_with_prefetch(tmp.path(), &["demo-target@1.0.0"]);

    specify_cmd().current_dir(tmp.path()).args(["adapters", "sync"]).assert().success();

    let lock = fs::read_to_string(tmp.path().join(".specify/adapters.lock")).expect("lock written");
    let digest = common::sha256_hex(&entry);
    assert_eq!(
        lock,
        format!("version: 1\nadapters:\n  demo-target@1.0.0: sha256:{digest}\n"),
        "sync pins the new identity in the committed lock"
    );

    let events = read_journal_normalized(tmp.path());
    assert_eq!(events.len(), 1, "one adapters.synced event: {events:?}");
    assert_eq!(events[0]["event"], "adapters.synced");
    assert_eq!(events[0]["payload"]["resolved"], 1);
    assert_eq!(events[0]["payload"]["fetched"], 0);
}

#[test]
fn frozen_miss_is_typed_exit_two() {
    // `--frozen` fetches nothing: a store miss aborts with the typed
    // `adapter-not-installed` (exit 2), naming the identity and the
    // literal sync command. Nothing lands in the store or the lock.
    let tmp = tempdir().unwrap();
    seed_project_with_prefetch(tmp.path(), &["demo-missing@3.2.1"]);

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapters", "sync", "--frozen"])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("process exited with a code");
    assert_eq!(code, 2, "frozen miss is a validation failure");
    let envelope: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stderr).expect("stderr is the JSON envelope");
    assert_eq!(envelope["error"], "adapter-not-installed");
    let message = envelope["message"].as_str().expect("message");
    assert!(message.contains("demo-missing@3.2.1"), "error names the identity: {message}");
    assert!(
        message.contains("specify adapters sync"),
        "error names the literal sync command: {message}"
    );
    assert!(
        !tmp.path().join(".specify/adapters.lock").exists(),
        "a frozen miss must not write the lock"
    );
}

#[test]
fn frozen_sync_never_writes_lock() {
    // `--frozen` is strictly read-only on the committed lock: a warm
    // store entry whose identity is new to the lock resolves and
    // verifies, but nothing is appended — the lock is never created
    // when absent, and survives byte-identical when present.
    let tmp = tempdir().unwrap();
    stage_store_component("demo-target", "1.0.0");
    seed_project_with_prefetch(tmp.path(), &["demo-target@1.0.0"]);

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapters", "sync", "--frozen"])
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["resolved"], 1);
    assert!(
        !tmp.path().join(".specify/adapters.lock").exists(),
        "a frozen sync must not create the lock"
    );

    // Pin the first identity un-frozen, then resolve a second,
    // new-to-the-lock identity frozen.
    specify_cmd().current_dir(tmp.path()).args(["adapters", "sync"]).assert().success();
    let lock = tmp.path().join(".specify/adapters.lock");
    let bytes = fs::read(&lock).expect("lock written");

    stage_store_component("demo-extra", "2.0.0");
    seed_project_with_prefetch(tmp.path(), &["demo-target@1.0.0", "demo-extra@2.0.0"]);
    specify_cmd().current_dir(tmp.path()).args(["adapters", "sync", "--frozen"]).assert().success();
    assert_eq!(
        fs::read(&lock).expect("lock bytes"),
        bytes,
        "a frozen sync resolving a new-to-the-lock identity must leave the lock byte-identical"
    );
}

/// Stage the echo *source*-adapter guest as a verified store entry —
/// the source-axis twin of `common::stage_store_component` (which
/// stages the target-axis echo bytes).
fn stage_store_source_component(name: &str, version: &str) -> std::path::PathBuf {
    let entry = common::isolated_adapter_store_root().join(format!("{name}@{version}.wasm"));
    fs::copy(common::fixture_source_component(name), &entry).expect("stage source store entry");
    let digest = common::sha256_hex(&entry);
    fs::write(
        common::isolated_adapter_store_root().join(format!("{name}@{version}.meta")),
        format!("tree_digest: sha256:{digest}\n"),
    )
    .expect("write store meta sidecar");
    entry
}

/// Parse the generated deployment manifest for `project` from the
/// per-project cache.
fn read_manifest(project: &Path) -> toml::Value {
    let path = common::expected_cache_dir(project).join("deployment").join("omnia.toml");
    toml::from_str(&fs::read_to_string(&path).expect("generated manifest exists"))
        .expect("generated manifest is valid TOML")
}

/// The `id` of every `[[guest]]` entry in `doc`, in document order.
fn guest_ids(doc: &toml::Value) -> Vec<String> {
    doc["guest"]
        .as_array()
        .expect("guest array")
        .iter()
        .map(|guest| guest["id"].as_str().expect("guest id").to_string())
        .collect()
}

#[test]
fn sync_regenerates_deployment_manifest() {
    // Sync regenerates the deployment manifest from the resolved set
    // into the per-project cache and names the path in its envelope:
    // one `[[guest]]` per resolved component pointing at its store
    // entry (axis sniffed from the component's exports), the resolved
    // core guest with the adapter-contract link allow-list, the
    // writable `"."` mount over the project dir, one `/mcp/<name>`
    // route per adapter, and the in-process transport. The bare
    // (unresolvable) `adapter: omnia` development name is skipped —
    // provisioning covers pins; the guest leg regenerates strictly.
    let tmp = tempdir().unwrap();
    let entry = stage_store_component("demo-target", "1.0.0");
    seed_project_with_prefetch(tmp.path(), &["demo-target@1.0.0"]);

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapters", "sync"])
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);
    let deployment = common::expected_cache_dir(tmp.path()).join("deployment");
    let manifest_path = deployment.join("omnia.toml");
    assert_eq!(
        body["manifest"],
        manifest_path.display().to_string(),
        "the envelope names the regenerated manifest path"
    );

    let doc = read_manifest(tmp.path());
    assert_eq!(guest_ids(&doc), ["workflow", "target:demo-target"]);
    let guests = doc["guest"].as_array().expect("guest array");
    assert_eq!(
        guests[0]["source"]["path"].as_str(),
        Some(common::workflow_guest_wasm().display().to_string().as_str()),
        "the core guest resolves through the SPECIFY_CORE_PATH development override"
    );
    assert_eq!(
        guests[0]["link"].as_array().map(Vec::len),
        Some(2),
        "the workflow guest links both adapter-contract interfaces"
    );
    assert_eq!(
        guests[1]["source"]["path"].as_str(),
        Some(entry.display().to_string().as_str()),
        "the adapter guest points at its global store entry"
    );
    let mount = &doc["mount"].as_array().expect("mount array")[0];
    assert_eq!(mount["name"].as_str(), Some("."));
    assert_eq!(
        mount["path"].as_str(),
        Some(
            fs::canonicalize(tmp.path()).expect("canonical project").display().to_string().as_str()
        ),
        "the project dir is the writable mount"
    );
    assert_eq!(mount["writable"].as_bool(), Some(true));
    let route = &doc["route"]["http"].as_array().expect("http routes")[0];
    assert_eq!(route["prefix"].as_str(), Some("/mcp/demo-target"));
    assert_eq!(route["guest"].as_str(), Some("target:demo-target"));
    assert_eq!(doc["transport"]["default"].as_str(), Some("in-process"));
}

#[test]
fn resync_picks_up_new_prefetch_pin() {
    // Regeneration is a full re-derivation: a prefetch pin added after
    // the first sync lands in the manifest as a new guest (with its
    // axis sniffed from the component — the source-axis echo bytes
    // yield a `source:` id).
    let tmp = tempdir().unwrap();
    stage_store_component("demo-target", "1.0.0");
    seed_project_with_prefetch(tmp.path(), &["demo-target@1.0.0"]);
    specify_cmd().current_dir(tmp.path()).args(["adapters", "sync"]).assert().success();
    assert_eq!(guest_ids(&read_manifest(tmp.path())), ["workflow", "target:demo-target"]);

    stage_store_source_component("demo-source", "2.0.0");
    seed_project_with_prefetch(tmp.path(), &["demo-target@1.0.0", "demo-source@2.0.0"]);
    specify_cmd().current_dir(tmp.path()).args(["adapters", "sync"]).assert().success();
    assert_eq!(
        guest_ids(&read_manifest(tmp.path())),
        ["workflow", "target:demo-target", "source:demo-source"],
        "the re-sync picks up the new pin"
    );
}

#[test]
fn relocated_store_reflected_in_manifest() {
    // RFC-65 AC9: relocating the store via `SPECIFY_ADAPTER_STORE`
    // changes nothing but the root path — the generated manifest
    // follows the resolved entries into the new root.
    let tmp = tempdir().unwrap();
    let relocated = tempdir().unwrap();
    let staged = stage_store_component("demo-target", "1.0.0");
    let entry = relocated.path().join("demo-target@1.0.0.wasm");
    fs::copy(&staged, &entry).expect("stage entry in the relocated store");
    fs::copy(
        common::isolated_adapter_store_root().join("demo-target@1.0.0.meta"),
        relocated.path().join("demo-target@1.0.0.meta"),
    )
    .expect("stage sidecar in the relocated store");
    seed_project_with_prefetch(tmp.path(), &["demo-target@1.0.0"]);

    specify_cmd()
        .current_dir(tmp.path())
        .env("SPECIFY_ADAPTER_STORE", relocated.path())
        .args(["adapters", "sync"])
        .assert()
        .success();
    let doc = read_manifest(tmp.path());
    assert_eq!(
        doc["guest"].as_array().expect("guest array")[1]["source"]["path"].as_str(),
        Some(entry.display().to_string().as_str()),
        "the manifest follows the relocated store entry"
    );
}

#[test]
fn sync_hydrates_core_without_override() {
    // RFC-65 move 4: with no development override, the core identity
    // `core@<binary version>` joins the hydration set ahead of the
    // declared adapters. Frozen against an empty store, the miss is
    // the typed `adapter-not-installed` naming the core identity;
    // with the entry staged, the sync resolves it as the leading row
    // and pins it into the committed lock.
    let tmp = tempdir().unwrap();
    seed_project_with_prefetch(tmp.path(), &[]);
    let version = env!("CARGO_PKG_VERSION");
    let empty_store = tempdir().unwrap();

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .env_remove("SPECIFY_CORE_PATH")
        .env("SPECIFY_ADAPTER_STORE", empty_store.path())
        .args(["--format", "json", "adapters", "sync", "--frozen"])
        .assert()
        .failure();
    let code = assert.get_output().status.code().expect("process exited with a code");
    assert_eq!(code, 2, "a frozen core miss is a validation failure");
    let envelope: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stderr).expect("stderr is the JSON envelope");
    assert_eq!(envelope["error"], "adapter-not-installed");
    let message = envelope["message"].as_str().expect("message");
    assert!(message.contains(&format!("core@{version}")), "error names the core: {message}");

    let store = tempdir().unwrap();
    let entry = store.path().join(format!("core@{version}.wasm"));
    fs::copy(common::workflow_guest_wasm(), &entry).expect("stage core store entry");
    let digest = common::sha256_hex(&entry);
    fs::write(
        store.path().join(format!("core@{version}.meta")),
        format!("tree_digest: sha256:{digest}\n"),
    )
    .expect("write core meta sidecar");

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .env_remove("SPECIFY_CORE_PATH")
        .env("SPECIFY_ADAPTER_STORE", store.path())
        .args(["--format", "json", "adapters", "sync"])
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["resolved"], 1);
    assert_eq!(body["fetched"], 0, "a warm store fetches nothing");
    assert_eq!(body["adapters"][0]["name"], "core");
    assert_eq!(body["adapters"][0]["version"], version);

    let lock = fs::read_to_string(tmp.path().join(".specify/adapters.lock")).expect("lock written");
    assert!(
        lock.contains(&format!("core@{version}: sha256:{digest}")),
        "sync pins the core identity in the committed lock: {lock}"
    );
}

#[test]
fn no_project_is_not_initialized() {
    // Outside any `.specify/` project, sync fails with the standard
    // project-not-found error every project-scoped verb raises.
    let tmp = tempdir().unwrap();

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapters", "sync"])
        .assert()
        .failure();
    let envelope: serde_json::Value =
        serde_json::from_slice(&assert.get_output().stderr).expect("stderr is the JSON envelope");
    assert_eq!(envelope["error"], "not-initialized");
}

#[test]
fn empty_declared_set_is_a_noop() {
    // A project with no pinned declarations (a bare development
    // `adapter:` name) hydrates nothing: empty resolved set, no lock,
    // no journal event.
    let tmp = tempdir().unwrap();
    seed_project_with_prefetch(tmp.path(), &[]);

    let assert = specify_cmd()
        .current_dir(tmp.path())
        .args(["--format", "json", "adapters", "sync"])
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);
    assert_eq!(body["resolved"], 0);
    assert_eq!(body["adapters"].as_array().map(Vec::len), Some(0));
    assert!(
        !tmp.path().join(".specify/adapters.lock").exists(),
        "an empty declared set writes no lock"
    );
    assert!(
        !tmp.path().join(".specify/journal.jsonl").exists(),
        "an empty declared set journals nothing"
    );
}

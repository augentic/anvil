//! CLI acceptance tests for `specify extension`.

use std::fmt::Write as _;
use std::fs;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use tempfile::{TempDir, tempdir};

use crate::common::{copy_dir, parse_json, repo_root, sha256_hex, specify_cmd};

fn fixtures_root() -> PathBuf {
    repo_root().join("tests").join("fixtures")
}

struct ToolFixtures {
    _tmp: TempDir,
    root: PathBuf,
}

impl ToolFixtures {
    /// Shared lazy fixture copy for tests that only READ the fixture
    /// trees. Tests that mutate their tree (manifest rewrites, files
    /// written under the project root) must call `new()` for a private
    /// copy instead.
    fn shared() -> &'static Self {
        static SHARED: OnceLock<ToolFixtures> = OnceLock::new();
        SHARED.get_or_init(Self::new)
    }

    fn new() -> Self {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();
        for name in ["tools-test-project", "tools-test-adp", "tools-test-project-adp"] {
            copy_dir(&fixtures_root().join(name), &root.join(name));
        }
        for path in [
            root.join("tools-test-project/.specify/project.yaml"),
            root.join("tools-test-project-adp/.specify/project.yaml"),
        ] {
            let text = fs::read_to_string(&path).expect("read fixture manifest");
            fs::write(&path, text.replace("/__SPECIFY_FIXTURE_ROOT__", &root.to_string_lossy()))
                .expect("write materialized manifest");
        }
        copy_dir(
            &root.join("tools-test-adp"),
            &root.join("tools-test-project-adp/adapters/targets/tools-test-adp"),
        );
        Self { _tmp: tmp, root }
    }

    fn project(&self) -> PathBuf {
        self.root.join("tools-test-project")
    }

    fn cap_project(&self) -> PathBuf {
        self.root.join("tools-test-project-adp")
    }

    fn adapter(&self) -> PathBuf {
        self.cap_project().join("adapters").join("targets").join("tools-test-adp")
    }

    fn project_wasm(&self, name: &str) -> PathBuf {
        self.project().join("wasm").join(format!("{name}.wasm"))
    }
}

fn cache_dir(label: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos =
        SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |duration| duration.as_nanos());
    let path = std::env::temp_dir()
        .join(format!("specify-tool-{label}-{}-{nanos}-{n}", std::process::id()));
    fs::create_dir_all(&path).expect("create cache dir");
    path
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.display())
}

fn tool_entry(
    name: &str, version: &str, source: &str, sha256: Option<&str>, permissions: Option<&str>,
) -> String {
    let mut entry = format!("  - name: {name}\n    version: {version}\n    source: \"{source}\"\n");
    if let Some(sha256) = sha256 {
        writeln!(entry, "    sha256: {sha256}").expect("write sha256 YAML line");
    }
    if let Some(permissions) = permissions {
        entry.push_str(permissions);
    }
    entry
}

fn project_manifest(tools: &str) -> String {
    format!("name: tools-test\nworkspace: true\ntools:\n{tools}")
}

fn adapter_project_manifest(tools: Option<&str>) -> String {
    let mut yaml = "name: tools-test-project-adp\nadapter: tools-test-adp\nrules: {}\n".to_string();
    if let Some(tools) = tools {
        yaml.push_str("tools:\n");
        yaml.push_str(tools);
    }
    yaml
}

fn write_project_manifest(project: &Path, yaml: &str) {
    fs::write(project.join(".specify/project.yaml"), yaml).expect("write project.yaml");
}

/// Rewrite the fixture adapter's `adapter.yaml` so its singular
/// `extension` declaration runs under `name`. The WASI component is the
/// committed `adapter.wasm` already in the fixture tree (RFC-48 D11), so
/// only the run handle changes.
fn write_adapter_extension(adapter: &Path, name: &str) {
    fs::write(
        adapter.join("adapter.yaml"),
        format!(
            "name: tools-test-adp\nversion: 1.0.0\naxis: target\nexecution: agent\nbriefs:\n  shape: briefs/shape.md\n  build: briefs/build.md\n  merge: briefs/merge.md\ndescription: Fixture adapter for installed-tree extension acceptance tests\nextension:\n  name: {name}\n"
        ),
    )
    .expect("write adapter.yaml");
}

fn run_json_failure(project: &Path, cache: &Path, args: &[&str], code: i32) -> Value {
    let assert = specify_cmd()
        .current_dir(project)
        .env("SPECIFY_EXTENSIONS_CACHE", cache)
        .args(["--format", "json"])
        .args(args)
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(code));
    parse_json(&assert.get_output().stderr)
}

fn assert_validation_rule(value: &Value, rule_id: &str) {
    // Extension-manifest validation failures surface as a payload-free
    // `Error::Validation` keyed on the first failing rule id; the wire
    // `error` discriminant is that rule id directly.
    assert_eq!(value["error"], rule_id, "{value}");
    assert_eq!(value["exit-code"], 2, "{value}");
}

#[test]
fn help_lists_active_verbs() {
    // `tool --help` must exit 0; the verb inventory is asserted via the
    // contract dump rather than exact clap wording.
    specify_cmd().args(["extension", "--help"]).assert().success();
    let verbs = crate::common::contract_dump_verbs(&["extension"]);
    for verb in ["run", "fetch", "gc", "schema"] {
        assert!(verbs.iter().any(|v| v == verb), "tool must declare `{verb}`, got: {verbs:?}");
    }
}

#[test]
fn gc_prunes_orphaned_cache_entry() {
    // Install `echo` into the cache via `tool run`, then drop it from
    // the manifest so the cached version is no longer referenced by the
    // live merged inventory; `tool gc` must remove the orphaned
    // `<scope>/echo/0.1.0/` directory.
    let fixtures = ToolFixtures::new();
    let project = fixtures.project();
    let cache = cache_dir("gc-prune");

    specify_cmd()
        .current_dir(&project)
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["extension", "run", "echo", "--", "seed"])
        .assert()
        .success();
    let echo_dir = cache.join("project--tools-test/echo/0.1.0");
    assert!(echo_dir.is_dir(), "tool run must populate the cache before gc");

    // Drop every declared tool so the cached `echo` becomes an orphan.
    write_project_manifest(&project, "name: tools-test\nworkspace: true\n");

    let assert = specify_cmd()
        .current_dir(&project)
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["--format", "json", "extension", "gc"])
        .assert()
        .success();
    let body = parse_json(&assert.get_output().stdout);
    let removed = body["removed"].as_array().expect("removed array");
    assert!(
        removed.iter().any(|p| p.as_str().is_some_and(|s| s.contains("echo/0.1.0"))),
        "gc must report the orphaned echo cache dir as removed, got: {body}"
    );
    assert!(!echo_dir.exists(), "gc must delete the orphaned cache directory");
}

#[test]
fn manifest_validation_reports_rule_ids() {
    let fixtures = ToolFixtures::new();
    let project = fixtures.project();
    let cache = cache_dir("project-validation");
    let source = file_uri(&fixtures.project_wasm("echo"));
    let cases = [
        (tool_entry("BadName", "0.1.0", &source, None, None), "tool.name-format"),
        (tool_entry("echo", "not-semver", &source, None, None), "tool.version-is-semver"),
        (tool_entry("echo", "0.1.0", "https://", None, None), "tool.source-is-supported-uri"),
        (tool_entry("echo", "0.1.0", &source, Some("ABC"), None), "tool.sha256-format"),
        (
            tool_entry(
                "echo",
                "0.1.0",
                &source,
                None,
                Some("    permissions:\n      read:\n        - \"$PROJECT_DIR/../inputs\"\n"),
            ),
            "tool.permission-path-form",
        ),
        (
            tool_entry(
                "echo",
                "0.1.0",
                &source,
                None,
                Some("    permissions:\n      write:\n        - \"$PROJECT_DIR/.specify\"\n"),
            ),
            "tool.lifecycle-state-write-denied",
        ),
        (
            tool_entry(
                "echo",
                "0.1.0",
                &source,
                None,
                Some("    permissions:\n      read:\n        - \"$CAPABILITY_DIR/templates\"\n"),
            ),
            "tool.capability-dir-out-of-scope",
        ),
    ];

    for (entry, rule_id) in cases {
        write_project_manifest(&project, &project_manifest(&entry));
        let value = run_json_failure(&project, &cache, &["extension", "run", "echo"], 2);
        assert_validation_rule(&value, rule_id);
    }
}

#[test]
fn cache_miss_hit_and_override_observable() {
    let fixtures = ToolFixtures::new();
    let project = fixtures.project();
    let cache = cache_dir("cache-flow");

    let first = specify_cmd()
        .current_dir(&project)
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["extension", "run", "echo", "--", "hello", "world"])
        .assert()
        .success();
    let stdout = String::from_utf8(first.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("echo: hello world"), "{stdout}");
    assert!(stdout.contains("CAPABILITY_DIR=<unset>"), "{stdout}");
    assert!(stdout.contains("PATH=<unset>"), "{stdout}");

    let sidecar = cache.join("project--tools-test/echo/0.1.0/meta.yaml");
    assert!(sidecar.is_file(), "SPECIFY_EXTENSIONS_CACHE override should receive cache writes");
    let fetched_at = fs::read_to_string(&sidecar).expect("read sidecar");

    specify_cmd()
        .current_dir(&project)
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["extension", "run", "echo", "--", "hello", "world"])
        .assert()
        .success();
    assert_eq!(
        fs::read_to_string(&sidecar).expect("read sidecar after hit"),
        fetched_at,
        "cache hit must not rewrite fetched-at"
    );

    let pinned = tool_entry(
        "echo",
        "0.1.0",
        &file_uri(&fixtures.project_wasm("echo")),
        Some(&sha256_hex(&fixtures.project_wasm("echo"))),
        None,
    );
    write_project_manifest(&project, &project_manifest(&pinned));

    specify_cmd()
        .current_dir(&project)
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["extension", "fetch", "echo"])
        .assert()
        .success();
    let refetched = fs::read_to_string(&sidecar).expect("read refetched sidecar");
    assert!(refetched.contains("sha256:"), "{refetched}");
}

#[test]
fn digest_mismatch_fails_before_install() {
    let fixtures = ToolFixtures::new();
    let project = fixtures.project();
    let cache = cache_dir("digest-mismatch");
    let wrong_sha = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
    let entry = tool_entry(
        "echo",
        "0.1.0",
        &file_uri(&fixtures.project_wasm("echo")),
        Some(wrong_sha),
        None,
    );
    write_project_manifest(&project, &project_manifest(&entry));

    let value = run_json_failure(&project, &cache, &["extension", "run", "echo"], 1);
    assert_eq!(value["error"], "tool-resolver", "{value}");
    assert!(
        !cache.join("project--tools-test/echo/0.1.0/module.wasm").exists(),
        "digest mismatch must not install staged bytes"
    );
}

#[test]
fn local_path_source_runs_without_file_uri() {
    let fixtures = ToolFixtures::new();
    let project = fixtures.project();
    let cache = cache_dir("local-path");
    let source = fixtures.project_wasm("echo").display().to_string();
    let entry = tool_entry("echo", "0.1.0", &source, None, None);
    write_project_manifest(&project, &project_manifest(&entry));

    let assert = specify_cmd()
        .current_dir(&project)
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["extension", "run", "echo", "--", "local"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("echo: local"), "{stdout}");
}

#[test]
fn https_network_failure_is_typed() {
    let fixtures = ToolFixtures::new();
    let project = fixtures.project();
    let cache = cache_dir("network-failure");
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local port");
    let url = format!("https://{}/echo.wasm", listener.local_addr().expect("addr"));
    drop(listener);
    let entry = tool_entry("echo", "0.1.0", &url, None, None);
    write_project_manifest(&project, &project_manifest(&entry));

    let value = run_json_failure(&project, &cache, &["extension", "run", "echo"], 1);
    let code = value["error"].as_str().expect("error code");
    assert!(
        matches!(code, "tool-network-other" | "tool-network-timeout" | "tool-network-malformed"),
        "{value}"
    );
    assert!(value["message"].as_str().expect("message").contains(&url), "{value}");
}

#[test]
fn invalid_wasm_runtime_failure_is_typed() {
    let fixtures = ToolFixtures::new();
    let project = fixtures.project();
    let cache = cache_dir("runtime-failure");
    let invalid_wasm = project.join("not-a-component.wasm");
    fs::write(&invalid_wasm, b"not a wasm component").expect("write invalid wasm");
    let entry = tool_entry("echo", "0.1.0", &file_uri(&invalid_wasm), None, None);
    write_project_manifest(&project, &project_manifest(&entry));

    let value = run_json_failure(&project, &cache, &["extension", "run", "echo"], 1);
    assert_eq!(value["error"], "tool-runtime", "{value}");
}

#[test]
fn allowed_fs_reads_and_writes_declared() {
    let fixtures = ToolFixtures::new();
    let project = fixtures.project();
    let cache = cache_dir("allowed-fs");

    let read = specify_cmd()
        .current_dir(&project)
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["extension", "run", "read-only"])
        .assert()
        .success();
    let stdout = String::from_utf8(read.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("read-only: fixture-probe"), "{stdout}");

    let result = project.join("outputs/result.txt");
    drop(fs::remove_file(&result));
    let write = specify_cmd()
        .current_dir(&project)
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["extension", "run", "read-write"])
        .assert()
        .success();
    let stdout = String::from_utf8(write.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("read-write: derived: fixture-probe"), "{stdout}");
    assert_eq!(fs::read_to_string(result).expect("read output"), "derived: fixture-probe");
}

#[test]
fn denied_fs_lifecycle_fail_pre_guest() {
    let fixtures = ToolFixtures::new();
    let project = fixtures.project();
    let cache = cache_dir("denied-fs");
    let denied = tool_entry(
        "read-only",
        "0.1.0",
        &file_uri(&fixtures.project_wasm("read-only")),
        None,
        Some("    permissions:\n      read:\n        - \"/etc\"\n"),
    );
    write_project_manifest(&project, &project_manifest(&denied));

    let value = run_json_failure(&project, &cache, &["extension", "run", "read-only"], 2);
    assert_validation_rule(&value, "tool-permission-denied");

    let lifecycle = tool_entry(
        "read-write",
        "0.1.0",
        &file_uri(&fixtures.project_wasm("read-write")),
        None,
        Some("    permissions:\n      write:\n        - \"$PROJECT_DIR/.specify\"\n"),
    );
    write_project_manifest(&project, &project_manifest(&lifecycle));
    let value = run_json_failure(&project, &cache, &["extension", "run", "read-write"], 2);
    assert_validation_rule(&value, "tool.lifecycle-state-write-denied");
}

#[test]
fn adapter_non_zero_exit_caches_by_scope() {
    // Read-only against the fixture tree (the cache dir is private),
    // so it can use the shared copy.
    let fixtures = ToolFixtures::shared();
    let cache = cache_dir("exit-seven");
    let assert = specify_cmd()
        .current_dir(fixtures.cap_project())
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["extension", "run", "exit-seven"])
        .assert()
        .failure();
    assert_eq!(assert.get_output().status.code(), Some(7));
    assert!(cache.join("adapter--target--tools-test-adp/exit-seven/1.0.0/module.wasm").is_file());
}

#[test]
fn name_collision_project_scope_wins() {
    let fixtures = ToolFixtures::new();
    let project = fixtures.cap_project();
    let cache = cache_dir("collision");
    let project_echo =
        tool_entry("echo", "0.1.0", &file_uri(&fixtures.project_wasm("echo")), None, None);
    // Point the adapter's singular extension at the run handle `echo`,
    // colliding with the project-scope `echo`; project scope must win.
    write_project_manifest(&project, &adapter_project_manifest(Some(&project_echo)));
    write_adapter_extension(&fixtures.adapter(), "echo");

    let run = specify_cmd()
        .current_dir(&project)
        .env("SPECIFY_EXTENSIONS_CACHE", &cache)
        .args(["extension", "run", "echo", "--", "project-wins"])
        .assert()
        .success();
    let stdout = String::from_utf8(run.get_output().stdout.clone()).expect("utf8 stdout");
    assert!(stdout.contains("echo: project-wins"), "{stdout}");
    let stderr = String::from_utf8(run.get_output().stderr.clone()).expect("utf8 stderr");
    assert!(stderr.contains("tool-name-collision"), "{stderr}");
}

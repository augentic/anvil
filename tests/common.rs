//! Helpers shared across the binary's integration tests.
//!
//! Each test file `mod common;` to pull these in (cargo's "include
//! shared module" idiom for `tests/`). Some test files use only a
//! subset; the module-root `#![allow(dead_code, unused_imports, ...)]`
//! below keeps the unused-helper warnings off without per-item
//! attributes (`allow`, not `expect`: fulfilment varies per binary).

#![allow(
    dead_code,
    unused_imports,
    reason = "test helpers shared across integration test binaries; not every binary uses every helper or re-export"
)]

#[path = "fs_git.rs"]
mod fs_git;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use assert_cmd::Command;
pub use fs_git::{GIT_ENV, copy_dir, run_git};
use serde_json::Value;
use specify_error::Result;
use tempfile::{TempDir, tempdir};

/// Panic with a descriptive message when a handler returned an error.
///
/// The shared `Result<()>`-shaped success check for integration tests.
#[track_caller]
pub fn assert_ok(result: Result<()>, what: &str) {
    result.unwrap_or_else(|err| panic!("{what} failed: {err}"));
}

/// Path to the repo root for the `specify` crate (where the
/// integration tests live).
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Convenience pointer to the staged `omnia.wasm` fixture component
/// used as the canonical positional argument for `specify init`
/// (RFC-64: an adapter is one component file). The bytes are the echo
/// target-adapter guest; the `omnia` filename gives the project its
/// canonical target name.
pub fn omnia_component() -> PathBuf {
    fixture_component("omnia")
}

/// Stage the echo target-adapter guest under
/// `target/test-components/<name>.wasm` and return the path. The
/// filename carries the adapter identity (`specify init` derives the
/// adapter name from the component file stem), and the echo guest's
/// `describe` branches on the routed id, so one binary stands in for
/// several fixture adapters (`omnia`, `vectis-platforms`,
/// `adapter-limited`, …).
///
/// # Panics
///
/// Panics when the guest build or the staging copy fails.
pub fn fixture_component(name: &str) -> PathBuf {
    stage_named_component(name, &echo_target_guest_wasm())
}

/// Source-axis twin of [`fixture_component`]: stages the echo
/// source-adapter guest bytes under the given adapter name.
pub fn fixture_source_component(name: &str) -> PathBuf {
    stage_named_component(name, &echo_source_guest_wasm())
}

fn stage_named_component(name: &str, built: &Path) -> PathBuf {
    let staged_dir = cargo_target_dir().join("test-components");
    let staged = staged_dir.join(format!("{name}.wasm"));
    let bytes = fs::read(built).expect("read built echo guest");
    if fs::read(&staged).is_ok_and(|current| current == bytes) {
        return staged;
    }
    fs::create_dir_all(&staged_dir).expect("create test-components dir");
    // Atomic temp-then-rename: concurrent test processes may stage the
    // same fixture, and a reader must never observe a half-written file.
    let tmp = staged_dir.join(format!(".{name}.{}.tmp", std::process::id()));
    fs::write(&tmp, &bytes).expect("write staged component");
    fs::rename(&tmp, &staged).expect("publish staged component");
    staged
}

/// Build (once per test process) the echo target-adapter guest and
/// return the artifact path. Cargo's own build lock serializes
/// concurrent invocations across test binaries.
fn echo_target_guest_wasm() -> PathBuf {
    use std::sync::OnceLock;
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT
        .get_or_init(|| build_echo_guest("specify-echo-target-guest", "specify_echo_target_guest"))
        .clone()
}

/// Build (once per test process) the echo source-adapter guest and
/// return the artifact path.
fn echo_source_guest_wasm() -> PathBuf {
    use std::sync::OnceLock;
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| build_echo_guest("specify-echo-guest", "specify_echo_guest")).clone()
}

fn build_echo_guest(package: &str, artifact_stem: &str) -> PathBuf {
    let status = std::process::Command::new("cargo")
        .env("CARGO_TARGET_DIR", cargo_target_dir())
        .args(["build", "-p", package, "--target", "wasm32-wasip2"])
        .current_dir(repo_root())
        .status()
        .expect("spawning echo guest build");
    assert!(status.success(), "echo guest build failed with status {status}");
    cargo_target_dir().join("wasm32-wasip2").join("debug").join(format!("{artifact_stem}.wasm"))
}

/// The cargo target dir this test binary was built into (the test exe
/// sits at `<target>/<profile>/deps/<exe>`).
fn cargo_target_dir() -> PathBuf {
    let test_exe = std::env::current_exe().expect("test executable has a path");
    test_exe
        .ancestors()
        .nth(3)
        .expect("test exe sits at <target>/<profile>/deps/<exe>")
        .to_path_buf()
}

/// Build a fresh `assert_cmd::Command` for the locally-built `specify`
/// binary. Scrubs the ambient `SPECIFY_*` env overrides so an
/// operator shell mid-workspace-run (exported `SPECIFY_PLAN_DIR`)
/// cannot skew test plan resolution. Pins the wasmtime compilation
/// cache to one repo-local directory: tests isolate `SPECIFY_EXTENSIONS_CACHE`
/// per test, which would otherwise defeat compiled-component reuse and
/// make every WASI-dispatching test pay the full Cranelift compile.
pub fn specify_cmd() -> Command {
    let mut cmd = Command::cargo_bin("specify").expect("cargo_bin(specify)");
    cmd.env_remove("SPECIFY_PLAN_DIR");
    cmd.env_remove("SPECIFY_FORMAT");
    cmd.env("SPECIFY_WASMTIME_CACHE", repo_root().join("target").join("wasmtime-cache"));
    // Pin the out-of-tree adapter/codex cache into a per-process temp
    // root so the developer's real OS cache is never touched and the
    // cache lands somewhere the test can locate via `expected_cache_dir`.
    cmd.env("SPECIFY_PROJECT_CACHE", isolated_cache_root());
    // Pin the persistent Git mirror root into a per-process temp root so
    // remote-peer materialisation never touches the developer's real OS
    // cache and mirror reuse is observable across invocations in one test.
    cmd.env("SPECIFY_MIRROR_CACHE", isolated_mirror_root());
    // Pin the global adapter store into a per-process temp root so
    // pinned-identity resolution never reads (or writes) the
    // developer's real content-addressed store.
    cmd.env("SPECIFY_ADAPTER_CACHE", isolated_adapter_store_root());
    cmd
}

/// Per-process out-of-tree global adapter-store root, matching the
/// `SPECIFY_ADAPTER_CACHE` override [`specify_cmd`] pins.
pub fn isolated_adapter_store_root() -> &'static Path {
    use std::sync::OnceLock;
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let dir =
            std::env::temp_dir().join(format!("specify-adapter-store-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create isolated adapter store root");
        dir
    })
}

/// Stage the echo target guest as a verified global-store entry
/// `<store>/<name>@<version>.wasm` (bytes + digest sidecar) inside the
/// isolated store root, so pinned identities (`<name>@<version>`)
/// resolve in tests.
pub fn stage_store_component(name: &str, version: &str) -> PathBuf {
    let entry = isolated_adapter_store_root().join(format!("{name}@{version}.wasm"));
    fs::copy(fixture_component(name), &entry).expect("stage store component");
    let digest = specify_schema::digest::sha256_hex(&fs::read(&entry).expect("read store entry"));
    fs::write(
        isolated_adapter_store_root().join(format!("{name}@{version}.meta")),
        format!("tree_digest: sha256:{digest}\n"),
    )
    .expect("write store meta sidecar");
    entry
}

/// Per-process out-of-tree Git-mirror root. One temp directory per test
/// binary process, isolated from other tests and from `~/.cache`.
pub fn isolated_mirror_root() -> &'static Path {
    use std::sync::OnceLock;
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("specify-mirror-cache-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create isolated mirror cache root");
        dir
    })
}

/// Per-process out-of-tree project-cache root. One temp directory per
/// test binary process (nextest runs each test in its own process), so
/// every `specify` invocation in a test shares one cache, isolated from
/// other tests and from `~/.cache`.
pub fn isolated_cache_root() -> &'static Path {
    use std::sync::OnceLock;
    static ROOT: OnceLock<PathBuf> = OnceLock::new();
    ROOT.get_or_init(|| {
        let dir =
            std::env::temp_dir().join(format!("specify-project-cache-{}", std::process::id()));
        fs::create_dir_all(&dir).expect("create isolated project cache root");
        dir
    })
}

/// The out-of-tree cache directory the binary resolves for `project_dir`
/// under the test's [`isolated_cache_root`]. Mirror of the production
/// resolver, so tests assert cache contents (`manifests/`, `codex/`)
/// without depending on the developer's OS cache.
pub fn expected_cache_dir(project_dir: &Path) -> PathBuf {
    specify_schema::cache::project_cache_dir_in(isolated_cache_root(), project_dir)
}

/// Stamp a phase outcome on `<project>/slices/<name>/metadata.yaml`
/// through the domain writer merge uses (`stamp_outcome`).
///
/// Integration tests call this directly because outcome inspection is no
/// longer exposed as CLI product surface.
pub fn stamp_slice_outcome(
    project: &Project, name: &str, phase: specify_workflow::adapter::TargetOperation,
    kind: specify_workflow::slice::OutcomeKind, summary: &str, context: Option<&str>,
) {
    use jiff::Timestamp;
    use specify_workflow::slice::actions as slice_actions;

    let slice_dir = project.slices_dir().join(name);
    slice_actions::stamp_outcome(
        &slice_dir,
        phase,
        kind,
        summary,
        context,
        Timestamp::from_str("2026-04-24T12:00:00Z").expect("fixed test timestamp"),
    )
    .expect("stamp outcome");
}

/// Subcommand names beneath the given command path (empty slice for
/// the top level), parsed from the `Commands:` section of clap's
/// `--help` output. The verb inventory help tests assert against
/// instead of exact clap description wording.
pub fn help_verbs(path: &[&str]) -> Vec<String> {
    let assert = specify_cmd().args(path).arg("--help").assert().success();
    let help = String::from_utf8(assert.get_output().stdout.clone()).expect("utf8 help output");
    let mut verbs = Vec::new();
    let mut in_commands = false;
    for line in help.lines() {
        if line.trim_end() == "Commands:" {
            in_commands = true;
            continue;
        }
        if in_commands {
            let Some(rest) = line.strip_prefix("  ") else {
                if line.trim().is_empty() {
                    continue;
                }
                break;
            };
            if rest.starts_with(' ') {
                continue;
            }
            if let Some(name) = rest.split_whitespace().next() {
                verbs.push(name.to_string());
            }
        }
    }
    assert!(!verbs.is_empty(), "no Commands: section parsed from `--help`:\n{help}");
    verbs
}

/// Hex-encoded SHA-256 of the bytes at `path`, used by every tool
/// integration suite to pin a `sha256:` digest into a manifest fixture.
///
/// # Panics
///
/// Panics if `path` cannot be read.
pub fn sha256_hex(path: &Path) -> String {
    let bytes = fs::read(path).expect("read bytes for sha256");
    specify_schema::digest::sha256_hex(&bytes)
}

/// Pinned RFC 3339 timestamp every journal-reading suite normalises
/// event `timestamp` fields to. CLI-driven emits stamp
/// `Timestamp::now()`; tests rewrite the value to this placeholder so
/// assertions (and goldens) stay deterministic across runs.
pub const FIXED_TIMESTAMP: &str = "2026-05-21T20:00:00Z";

/// Read `<root>/.specify/journal.jsonl`, returning one parsed `Value`
/// per non-blank line with every event's `timestamp` normalised to
/// [`FIXED_TIMESTAMP`].
///
/// This is the single home for the journal-reading + timestamp
/// normalisation pattern: callers that want structured journal
/// assertions parse the lines here and assert on fields, rather than
/// substring-matching raw JSON text.
///
/// # Panics
///
/// Panics if the journal file is missing or a line is not valid JSON.
pub fn read_journal_normalized(root: &Path) -> Vec<Value> {
    let path = root.join(".specify").join("journal.jsonl");
    let raw =
        fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()));
    raw.lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut value: Value = serde_json::from_str(line).expect("journal line is JSON");
            if let Value::Object(map) = &mut value
                && map.contains_key("timestamp")
            {
                map.insert("timestamp".to_string(), Value::String(FIXED_TIMESTAMP.to_string()));
            }
            value
        })
        .collect()
}

/// Parse a captured stdout buffer as JSON, panicking on UTF-8 or parse
/// errors with the offending text included for debugging.
///
/// # Panics
///
/// Panics if `stdout` is not UTF-8 or not valid JSON.
pub fn parse_json(stdout: &[u8]) -> Value {
    let text = std::str::from_utf8(stdout).expect("utf8 stdout");
    serde_json::from_str(text).unwrap_or_else(|err| panic!("stdout not JSON ({err}):\n{text}"))
}

/// Recursively snapshot every regular file under `root` as a
/// `relative-path -> bytes` map, so an upgrade's write set can be
/// asserted by diffing two snapshots.
///
/// # Panics
///
/// Panics if a directory cannot be read or a file cannot be loaded.
pub fn snapshot_tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(dir).expect("read_dir") {
            let entry = entry.expect("dir entry");
            let path = entry.path();
            if entry.file_type().expect("file_type").is_dir() {
                walk(root, &path, out);
            } else {
                let rel = path.strip_prefix(root).expect("strip prefix").to_path_buf();
                out.insert(rel, fs::read(&path).expect("read file"));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

/// Scaffold an empty workspace project in `tmp` via `specify init --workspace`.
///
/// # Panics
///
/// Panics if the `specify init` invocation does not exit 0.
pub fn init_workspace(tmp: &TempDir, name: &str) {
    specify_cmd()
        .current_dir(tmp.path())
        .args(["init"])
        .args(["--name", name, "--workspace"])
        .assert()
        .success();
}

/// Placeholder substituted in for the test's tempdir path before
/// comparing stdout against a checked-in golden.
pub const TEMPDIR_PLACEHOLDER: &str = "<TEMPDIR>";

/// String-replacement rule applied to every JSON string before golden
/// comparison.
pub struct Sub {
    pub from: String,
    pub to: &'static str,
}

impl Sub {
    pub fn new(from: impl Into<String>, to: &'static str) -> Self {
        Self {
            from: from.into(),
            to,
        }
    }
}

/// Substitutions covering every way the tempdir at `root` might appear
/// in stdout.
///
/// macOS canonicalises `/var/folders/...` to `/private/var/folders/...`
/// whenever a subcommand touches the filesystem, so both spellings are
/// stripped. Sorting by length descending guarantees the longer
/// canonical path is replaced first; otherwise the shorter raw path
/// would match inside the canonical one and leave a stray `/private`
/// prefix in the golden.
pub fn tempdir_subs(root: &Path) -> Vec<Sub> {
    let mut subs: Vec<Sub> = Vec::new();
    if let Some(raw) = root.to_str() {
        subs.push(Sub::new(raw.to_string(), TEMPDIR_PLACEHOLDER));
    }
    if let Ok(canonical) = fs::canonicalize(root)
        && let Some(canonical_str) = canonical.to_str()
        && Some(canonical_str) != root.to_str()
    {
        subs.push(Sub::new(canonical_str.to_string(), TEMPDIR_PLACEHOLDER));
    }
    subs.sort_by_key(|s| std::cmp::Reverse(s.from.len()));
    subs
}

/// Walk `value` recursively and replace every occurrence of
/// `sub.from` with `sub.to` in any contained string.
pub fn strip_substitutions(value: &mut Value, subs: &[Sub]) {
    match value {
        Value::String(s) => {
            for sub in subs {
                if s.contains(&sub.from) {
                    *s = s.replace(&sub.from, sub.to);
                }
            }
        }
        Value::Array(items) => {
            for item in items {
                strip_substitutions(item, subs);
            }
        }
        Value::Object(map) => {
            for (_k, v) in map.iter_mut() {
                strip_substitutions(v, subs);
            }
        }
        _ => {}
    }
}

/// Parse `stdout` as JSON and apply [`tempdir_subs`] for `root`.
///
/// # Panics
///
/// Panics if `stdout` is not UTF-8 or not valid JSON.
pub fn parse_stdout(stdout: &[u8], root: &Path) -> Value {
    parse_json_stream("stdout", stdout, root)
}

/// Mirror of [`parse_stdout`] for the stderr channel. Used by
/// failure tests, which write the error envelope to stderr in both
/// JSON and text formats.
///
/// # Panics
///
/// Panics if `stderr` is not UTF-8 or not valid JSON.
pub fn parse_stderr(stderr: &[u8], root: &Path) -> Value {
    parse_json_stream("stderr", stderr, root)
}

fn parse_json_stream(label: &str, bytes: &[u8], root: &Path) -> Value {
    let text = std::str::from_utf8(bytes).unwrap_or_else(|_| panic!("utf8 {label}"));
    let mut value: Value = serde_json::from_str(text)
        .unwrap_or_else(|err| panic!("{label} not JSON ({err}):\n{text}"));
    strip_substitutions(&mut value, &tempdir_subs(root));
    value
}

/// A throwaway `.specify/` project rooted in a tempdir, scaffolded by
/// running `specify init` with the in-repo Omnia adapter fixture.
///
/// Hoisted from the per-test-file `struct Project` harnesses
/// (`tests/slice.rs`, `tests/slice_merge.rs`, `tests/e2e.rs`,
/// `tests/adapter.rs`, `tests/workflow/`) so the same
/// `Project::init()` / `.stage_slice()` shape works
/// across every integration suite. Each test binary uses a different
/// subset; the module-level `#![expect(dead_code, ...)]` covers helpers
/// that any particular binary doesn't reach.
pub struct Project {
    _tmp: TempDir,
    root: PathBuf,
}

impl Project {
    /// Build a fresh tempdir and run `specify init <omnia.wasm>` with a
    /// default `--name`. The resulting project sits at the tempdir
    /// root; init mirrors the component into the project component
    /// cache, so subsequent invocations resolve the `omnia` target from
    /// there.
    pub fn init() -> Self {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();
        specify_cmd()
            .current_dir(&root)
            .args(["init"])
            .arg(omnia_component())
            .args(["--name", "test-proj"])
            .assert()
            .success();
        Self { _tmp: tmp, root }
    }

    /// Copy a fixture subtree under `tests/fixtures/e2e/<fixture>` into
    /// `.specify/slices/my-slice/` and return the slice directory path.
    pub fn stage_slice(&self, fixture: &str) -> PathBuf {
        let dst = self.root.join(".specify/slices/my-slice");
        fs::create_dir_all(&dst).expect("mkdir slice");
        copy_dir(&repo_root().join("tests/fixtures/e2e").join(fixture), &dst);
        dst
    }

    /// Path to the project root (the tempdir).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Path to `.specify/slices/` under the project root.
    pub fn slices_dir(&self) -> PathBuf {
        self.root.join(".specify/slices")
    }

    /// Path to `.specify/specs/` under the project root.
    pub fn specs_dir(&self) -> PathBuf {
        self.root.join(".specify/specs")
    }

    /// Path to the umbrella `plan.yaml` at the repo root.
    pub fn plan_path(&self) -> PathBuf {
        self.root.join("plan.yaml")
    }

    /// Seed `plan.yaml` (at the project root) with arbitrary YAML. Used
    /// by the change-umbrella tests to drive the file directly without
    /// going through the `plan create` verb.
    pub fn seed_plan(&self, yaml: &str) {
        fs::write(self.plan_path(), yaml).expect("write plan.yaml");
    }
}

/// Compare `actual` against the golden at `dir/name`, or rewrite that
/// golden when the `REGENERATE_GOLDENS` env var is set.
///
/// # Panics
///
/// Panics if the golden cannot be read, is not JSON, or differs from
/// `actual`.
#[expect(
    clippy::needless_pass_by_value,
    reason = "callers naturally pass owned `serde_json::Value` results"
)]
pub fn assert_golden_at(dir: &Path, name: &str, actual: Value) {
    let golden_path = dir.join(name);
    let rendered = serde_json::to_string_pretty(&actual).expect("pretty json");

    if std::env::var_os("REGENERATE_GOLDENS").is_some() {
        fs::create_dir_all(dir).expect("mkdir golden dir");
        fs::write(&golden_path, format!("{rendered}\n")).expect("write golden");
        return;
    }

    let expected_raw = fs::read_to_string(&golden_path).unwrap_or_else(|err| {
        panic!(
            "golden {} missing ({err}); regenerate via \
             REGENERATE_GOLDENS=1 cargo nextest run --test <binary>",
            golden_path.display()
        )
    });
    let expected: Value = serde_json::from_str(&expected_raw)
        .unwrap_or_else(|err| panic!("golden {} is not JSON: {err}", golden_path.display()));

    assert_eq!(
        actual,
        expected,
        "stdout diverged from golden {}\n--- actual ---\n{rendered}\n--- expected ---\n{expected_raw}",
        golden_path.display()
    );
}

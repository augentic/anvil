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

mod fs_git;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

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

/// Convenience pointer to the in-repo Omnia adapter fixture used as
/// the canonical positional argument for `specify init`.
pub fn omnia_schema_dir() -> PathBuf {
    repo_root().join("tests").join("fixtures").join("adapters").join("targets").join("omnia")
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
    cmd
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

/// Exclusive hold on `<root>/.specify/plan.lock` for the guard's
/// lifetime — stands in for the `/spec:execute` driver session now
/// that the plan-state-writing verbs (`plan next`, per-entry
/// `plan transition`, `slice merge run`) probe the lock and refuse an
/// unlocked driver. Dropping the guard closes the file
/// and releases the OS advisory lock.
pub struct PlanLock {
    _file: fs::File,
}

/// Acquire the plan lock at `<root>/.specify/plan.lock`, creating the
/// lockfile (and `.specify/`) as the skill snippet would.
pub fn hold_plan_lock(root: &Path) -> PlanLock {
    let dir = root.join(".specify");
    fs::create_dir_all(&dir).expect("mkdir .specify");
    let file = fs::File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(dir.join("plan.lock"))
        .expect("open plan.lock");
    file.lock().expect("acquire plan lock");
    PlanLock { _file: file }
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
/// the top level), read from `specify contract dump`. The robust verb
/// inventory help tests assert against instead of exact clap wording.
pub fn contract_dump_verbs(path: &[&str]) -> Vec<String> {
    let assert = specify_cmd().args(["--format", "json", "contract", "dump"]).assert().success();
    let dump: Value =
        serde_json::from_slice(&assert.get_output().stdout).expect("contract dump JSON");
    let mut node = &dump["commands"];
    for name in path {
        node = node["subcommands"]
            .as_array()
            .expect("subcommands array")
            .iter()
            .find(|n| n["name"] == *name)
            .unwrap_or_else(|| panic!("verb `{name}` missing from contract dump"));
    }
    node["subcommands"]
        .as_array()
        .expect("subcommands array")
        .iter()
        .map(|n| n["name"].as_str().expect("verb name").to_string())
        .collect()
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

/// Scaffold a minimal target-adapter project declaring a single WASI tool.
///
/// The caller owns `tmp`, keeping the project root alive for the test duration.
pub fn scaffold_tool_project(
    tmp: &TempDir, tool_name: &str, wasm_path: &Path,
) -> (PathBuf, PathBuf) {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);

    let project = tmp.path().to_path_buf();
    let adapter = project.join("adapters/targets/test-adp");
    let briefs = adapter.join("briefs");
    fs::create_dir_all(project.join(".specify")).expect("create .specify");
    fs::create_dir_all(&briefs).expect("create adapter briefs");

    let cache = std::env::temp_dir()
        .join(format!("specify-tool-schema-{tool_name}-{}-{n}", std::process::id()));
    fs::create_dir_all(&cache).expect("create cache");

    fs::write(
        project.join(".specify/project.yaml"),
        "name: schema-test\nadapter: test-adp\nrules: {}\n",
    )
    .expect("write project.yaml");
    fs::write(
        adapter.join("adapter.yaml"),
        format!(
            "name: test-adp\nversion: 1.0.0\naxis: target\nexecution: agent\nbriefs:\n  shape: briefs/shape.md\n  build: briefs/build.md\n  merge: briefs/merge.md\ndescription: Test adapter\nextension:\n  name: {tool_name}\n"
        ),
    )
    .expect("write adapter.yaml");
    for op in ["shape", "build", "merge"] {
        fs::write(
            briefs.join(format!("{op}.md")),
            format!("---\nid: {op}\ndescription: {op} brief\n---\n"),
        )
        .expect("write brief");
    }

    // The installed adapter tree carries its WASI component as the
    // committed `adapter.wasm` (RFC-48 D11); `specify extension run`
    // resolves the binary from there, not a retired `tools.yaml`.
    fs::copy(wasm_path, adapter.join("adapter.wasm")).expect("commit adapter.wasm");

    (project, cache)
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
/// `Project::init()` / `.with_schemas()` / `.stage_slice()` shape works
/// across every integration suite. Each test binary uses a different
/// subset; the module-level `#![expect(dead_code, ...)]` covers helpers
/// that any particular binary doesn't reach.
pub struct Project {
    _tmp: TempDir,
    root: PathBuf,
}

impl Project {
    /// Build a fresh tempdir and run `specify init <repo>/targets/omnia`
    /// with a default `--name`. The resulting project sits at the
    /// tempdir root.
    pub fn init() -> Self {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();
        specify_cmd()
            .current_dir(&root)
            .args(["init"])
            .arg(omnia_schema_dir())
            .args(["--name", "test-proj"])
            .assert()
            .success();
        Self { _tmp: tmp, root }
    }

    /// Initialise a project backed by a local fixture adapter dir.
    /// The fixture is mirrored into `<tmp>/adapters/targets/<name>/` so that
    /// subsequent `specify` invocations resolve it via the usual
    /// `adapters/targets/<name>/` probe.
    pub fn init_from_fixture(name: &str, fixture_dir: &Path) -> Self {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();
        copy_dir(fixture_dir, &root.join("adapters").join("targets").join(name));
        specify_cmd()
            .current_dir(&root)
            .args(["init"])
            .arg(root.join("adapters").join("targets").join(name))
            .args(["--name", "test-proj"])
            .assert()
            .success();
        Self { _tmp: tmp, root }
    }

    /// Mirror the in-repo `targets/omnia` tree into the project so any
    /// subcommand that resolves the target adapter can find it under
    /// the project's own `adapters/targets/` dir.
    #[must_use]
    pub fn with_schemas(self) -> Self {
        copy_dir(&omnia_schema_dir(), &self.root.join("adapters").join("targets").join("omnia"));
        self
    }

    /// Populate the cache instead of the local `targets/` tree so
    /// `TargetAdapter::resolve` picks the `AdapterLocation::Cached`
    /// branch.
    #[must_use]
    pub fn with_cached_schema(self) -> Self {
        let cached = expected_cache_dir(&self.root).join("manifests/targets/omnia");
        copy_dir(&omnia_schema_dir(), &cached);
        self
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

    /// Hold the project's plan lock for the guard's lifetime (the
    /// driver-session stand-in for `plan next` / per-entry
    /// `plan transition` / `slice merge run` invocations).
    pub fn hold_plan_lock(&self) -> PlanLock {
        hold_plan_lock(self.root())
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

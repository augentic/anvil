//! Shared test helpers for `workflow` integration tests.
//!
//! Centralises [`MockCmd`], a recorder that captures every invocation
//! and dispatches the response through a per-test closure. Pass it to
//! domain code as `&|cmd| mock.run(cmd)`.

#![expect(
    dead_code,
    reason = "shared test helpers; not every integration binary uses every helper"
)]
#![expect(
    clippy::unnecessary_wraps,
    reason = "mock dispatch closures share a Result<Output> signature for parity with real_cmd"
)]

use std::cell::RefCell;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Output};

/// One recorded invocation captured by [`MockCmd`].
#[derive(Debug, Clone)]
pub struct RecordedCall {
    pub program: String,
    pub args: Vec<String>,
    pub current_dir: Option<PathBuf>,
}

type Handler = Box<dyn FnMut(&RecordedCall) -> io::Result<Output>>;

/// In-process command recorder that delegates dispatch to `handler`.
#[expect(
    clippy::partial_pub_fields,
    reason = "tests inspect `calls` directly; `handler` is an implementation detail of the closure dispatch"
)]
pub struct MockCmd {
    handler: RefCell<Handler>,
    pub calls: RefCell<Vec<RecordedCall>>,
}

impl MockCmd {
    /// Build a `MockCmd` from a dispatch closure.
    pub fn new<F>(handler: F) -> Self
    where
        F: FnMut(&RecordedCall) -> io::Result<Output> + 'static,
    {
        Self {
            handler: RefCell::new(Box::new(handler)),
            calls: RefCell::new(Vec::new()),
        }
    }

    /// Record `cmd` and dispatch through the handler. Pass this method
    /// to domain code via `&|cmd| mock.run(cmd)`; the `&mut Command`
    /// expected by `CmdRunner` reborrows to `&Command` at the call.
    pub fn run(&self, cmd: &Command) -> io::Result<Output> {
        let recorded = RecordedCall {
            program: cmd.get_program().to_string_lossy().into_owned(),
            args: cmd.get_args().map(|a| a.to_string_lossy().into_owned()).collect(),
            current_dir: cmd.get_current_dir().map(PathBuf::from),
        };
        self.calls.borrow_mut().push(recorded.clone());
        (self.handler.borrow_mut())(&recorded)
    }
}

/// Produce a successful [`Output`] with `stdout` (no stderr).
pub fn ok_stdout(stdout: &str) -> io::Result<Output> {
    Ok(Output {
        status: success_status(),
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
    })
}

/// Produce a successful [`Output`] with no stdout or stderr.
pub fn ok_empty() -> io::Result<Output> {
    ok_stdout("")
}

/// Produce an [`Output`] whose exit status is failure with `stderr`.
pub fn fail_stderr(stderr: &str) -> io::Result<Output> {
    Ok(Output {
        status: failure_status(),
        stdout: Vec::new(),
        stderr: stderr.as_bytes().to_vec(),
    })
}

#[cfg(unix)]
fn success_status() -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(0)
}

#[cfg(unix)]
fn failure_status() -> ExitStatus {
    use std::os::unix::process::ExitStatusExt;
    ExitStatus::from_raw(1 << 8)
}

// `copy_dir` / `run_git` come from the workspace-shared helper file; see
// `tests/fs_git.rs` at the repo root. Each is re-exposed as a thin
// wrapper (rather than a `pub use`) so a binary that never reaches it sees
// `dead_code` — covered by the module-level expectation — instead of an
// unused `pub use`. The `workspace` area reaches `run_git` as
// `crate::common::run_git`.
#[path = "../../../../tests/fs_git.rs"]
mod fs_git;
pub fn copy_dir(src: &std::path::Path, dst: &std::path::Path) {
    fs_git::copy_dir(src, dst);
}
pub fn run_git(root: &std::path::Path, args: &[&str]) -> String {
    fs_git::run_git(root, args)
}

const CACHE_ENV: &str = "SPECIFY_PROJECT_CACHE";

/// Restores the previous `SPECIFY_PROJECT_CACHE` value on drop.
pub struct CacheGuard(Option<std::ffi::OsString>);

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

/// Pin the out-of-tree project cache root inside `dir` so adapter
/// cache writes are hermetic and auto-cleaned with the tempdir.
#[expect(unsafe_code, reason = "pin the cache-root env var into the test tempdir")]
pub fn scoped_cache(dir: &std::path::Path) -> CacheGuard {
    let prev = std::env::var_os(CACHE_ENV);
    // SAFETY: see `CacheGuard::drop` — single-process test isolation.
    unsafe { std::env::set_var(CACHE_ENV, dir.join("project-cache")) };
    CacheGuard(prev)
}

/// Out-of-tree cache directory for `project_dir` under the pinned root.
pub fn expected_cache_dir(project_dir: &std::path::Path) -> PathBuf {
    schema::cache::project_cache_dir(project_dir)
}

const STORE_ENV: &str = "SPECIFY_ADAPTER_STORE";

/// Restores the previous `SPECIFY_ADAPTER_STORE` value on drop.
pub struct StoreGuard(Option<std::ffi::OsString>);

impl Drop for StoreGuard {
    #[expect(unsafe_code, reason = "restore the store-root env var pinned for the test")]
    fn drop(&mut self) {
        // SAFETY: nextest runs each test in its own process, so no other
        // thread observes the env mutation for the guard's lifetime.
        unsafe {
            match self.0.take() {
                Some(prev) => std::env::set_var(STORE_ENV, prev),
                None => std::env::remove_var(STORE_ENV),
            }
        }
    }
}

/// Pin the global content-addressed adapter store root
/// directly at `dir` so install / resolve probes are hermetic and
/// auto-cleaned with the tempdir.
#[expect(unsafe_code, reason = "pin the store-root env var into the test tempdir")]
pub fn scoped_store(dir: &std::path::Path) -> StoreGuard {
    let prev = std::env::var_os(STORE_ENV);
    // SAFETY: see `StoreGuard::drop` — single-process test isolation.
    unsafe { std::env::set_var(STORE_ENV, dir) };
    StoreGuard(prev)
}

/// Component resolver with deterministic metadata answers for adapter
/// integration fixtures.
pub fn resolver() -> workflow::adapter::resolver::Component {
    use error::Error;
    use workflow::adapter::metadata::{Metadata, Request};

    fn stub(request: &Request<'_>) -> Result<Metadata, Error> {
        let raw = match request.adapter_id {
            "target:demo-target" => r#"{"specify-floor":"999.0.0"}"#,
            "target:bad-floor" => r#"{"specify-floor":"v1"}"#,
            "target:vectis" => {
                r#"{
                    "inputs": [
                        { "path": "tokens.yaml", "required": true },
                        { "path": "assets.yaml", "required": false }
                    ],
                    "platforms": {
                        "required": true,
                        "allowed": ["core", "ios", "android"],
                        "default": ["core", "ios", "android"]
                    }
                }"#
            }
            _ => "{}",
        };
        serde_json::from_str(raw).map_err(|err| Error::Diag {
            code: "adapter-metadata-failed",
            detail: format!("stub parse {}: {err}", request.adapter_id),
        })
    }

    workflow::adapter::resolver::Component::new(stub)
}

/// Stage a stub adapter component for `name` at the resolver's in-repo
/// development probe (`<root>/target/wasm32-wasip2/release/
/// <name>.wasm`) and register the JSON-body describe stub, so a
/// bare-name resolve inside `root` succeeds with an empty
/// `Metadata`.
pub fn stage_dev_component(root: &std::path::Path, name: &str) {
    let dev_dir = root.join("target/wasm32-wasip2/release");
    std::fs::create_dir_all(&dev_dir).expect("mkdir dev release dir");
    std::fs::write(dev_dir.join(format!("{}.wasm", name.replace('-', "_"))), "{}")
        .expect("write stub component");
}

// ---------------------------------------------------------------------------
// Project anchor + operation invocation
// ---------------------------------------------------------------------------

/// A throw-away project tree the verbs run against: the provider
/// anchor points at its root, and the derived project cache is pinned
/// beneath it so cache writes are hermetic.
#[expect(
    clippy::partial_pub_fields,
    reason = "tests read `root` directly; the tempdir handle is a lifetime detail"
)]
#[derive(Clone)]
pub struct Project {
    _tmp: std::sync::Arc<tempfile::TempDir>,
    pub root: PathBuf,
}

impl workflow::handler::Anchor for Project {
    fn project_root(&self) -> &std::path::Path {
        &self.root
    }
}

impl workflow::adapter::Resolver for Project {
    fn resolve_source(
        &self, adapter_ref: &workflow::adapter::AdapterRef, project_dir: &std::path::Path,
    ) -> Result<workflow::adapter::ResolvedSource, error::Error> {
        workflow::adapter::Resolver::resolve_source(&resolver(), adapter_ref, project_dir)
    }

    fn resolve_target(
        &self, adapter_ref: &workflow::adapter::AdapterRef, project_dir: &std::path::Path,
    ) -> Result<workflow::adapter::ResolvedTarget, error::Error> {
        workflow::adapter::Resolver::resolve_target(&resolver(), adapter_ref, project_dir)
    }
}

impl Project {
    /// A bare directory — nothing scaffolded (the scaffold-leg input).
    #[expect(unsafe_code, reason = "pin the cache-root env var into the test tempdir")]
    pub fn bare() -> Self {
        let tmp = tempfile::TempDir::new().expect("tempdir");
        let root = tmp.path().canonicalize().expect("canonical tempdir");
        // SAFETY: nextest runs each test in its own process, so no
        // other thread observes the env mutation.
        unsafe { std::env::set_var(CACHE_ENV, root.join("project-cache")) };
        std::env::set_current_dir(&root).expect("enter project root");
        Self {
            _tmp: std::sync::Arc::new(tmp),
            root,
        }
    }

    /// An initialised project (`.specify/project.yaml` present).
    pub fn initialised() -> Self {
        let project = Self::bare();
        std::fs::create_dir_all(project.root.join(".specify")).expect("mkdir .specify");
        std::fs::write(
            project.root.join(".specify/project.yaml"),
            "name: demo\nadapter: demo\nrules: {}\n",
        )
        .expect("write project.yaml");
        project
    }
}

/// Invoke one operation against the project anchor.
pub async fn run<R, B>(project: &Project, input: R::Input) -> Result<B, workflow::handler::Error>
where
    R: omnia_guest::api::operation::Operation<
            Project,
            Output = B,
            Error = workflow::handler::Error,
        >,
    B: Send,
{
    omnia_guest::api::invoke::Invoker::new("specify", project.clone())
        .invoke::<R>(omnia_guest::api::invocation::Invocation::new(input))
        .await
}

// ---------------------------------------------------------------------------
// Plan fixtures
// ---------------------------------------------------------------------------

/// Reduced-state reproduction of the plan execution §"The Plan"
/// fixture. There is no per-entry `failed`, `blocked`, or `skipped`
/// state — entries either move forward or stay where they are.
pub const PLAN_EXAMPLE_YAML: &str = r"name: platform-v2
sources:
  monolith:
    adapter: demo-source
    path: /path/to/legacy-codebase
  orders:
    adapter: demo-source
    path: git@github.com:org/orders-service.git
  payments:
    adapter: demo-source
    path: git@github.com:org/payments-service.git
  frontend:
    adapter: demo-source
    path: git@github.com:org/web-app.git
slices:
  - name: user-registration
    project: platform
    sources: [monolith]
    status: done
  - name: email-verification
    project: platform
    sources: [monolith]
    depends-on: [user-registration]
    status: in-progress
  - name: registration-duplicate-email-crash
    project: platform
    description: >
      Duplicate email submission returns 500 instead of 409.
      Discovered during email-verification extraction.
    status: pending
  - name: notification-preferences
    project: platform
    depends-on: [user-registration]
    description: >
      Greenfield — user-facing notification channel and frequency settings.
    status: pending
  - name: extract-shared-validation
    project: platform
    description: >
      Pull duplicated input validation into a shared validation crate
      before building checkout-flow.
    depends-on: [email-verification]
    status: pending
  - name: product-catalog
    project: platform
    sources: [monolith]
    depends-on: [extract-shared-validation]
    status: pending
  - name: shopping-cart
    project: platform
    sources: [orders]
    depends-on: [product-catalog, user-registration]
    status: pending
  - name: checkout-api
    project: platform
    sources: [payments]
    depends-on: [shopping-cart]
    status: pending
  - name: checkout-ui
    project: platform
    sources: [frontend]
    depends-on: [checkout-api]
    status: pending
";

/// A minimal in-memory plan named `test` wrapping `changes`.
pub fn plan_with_changes(changes: Vec<workflow::change::Entry>) -> workflow::change::Plan {
    workflow::change::Plan {
        name: "test".into(),
        lifecycle: workflow::change::Lifecycle::Pending,
        sources: std::collections::BTreeMap::new(),
        entries: changes,
    }
}

/// A minimal plan entry bound to project `default`.
pub fn change(name: &str, status: workflow::change::Status) -> workflow::change::Entry {
    workflow::change::Entry {
        name: name.into(),
        project: Some("default".into()),
        status,
        depends_on: vec![],
        sources: vec![],
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: workflow::change::SliceAuthorityOverride::default(),
    }
}

/// [`change`] plus a `depends-on` list.
pub fn change_with_deps(
    name: &str, status: workflow::change::Status, deps: &[&str],
) -> workflow::change::Entry {
    let mut e = change(name, status);
    e.depends_on = deps.iter().map(|s| (*s).into()).collect();
    e
}

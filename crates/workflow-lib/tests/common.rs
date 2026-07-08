//! Shared test helpers for `specify-workflow` integration tests.
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
#[path = "../../../tests/fs_git.rs"]
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

/// Pin the out-of-tree project cache root inside `dir` so adapter /
/// codex cache writes are hermetic and auto-cleaned with the tempdir.
#[expect(unsafe_code, reason = "pin the cache-root env var into the test tempdir")]
pub fn scoped_cache(dir: &std::path::Path) -> CacheGuard {
    let prev = std::env::var_os(CACHE_ENV);
    // SAFETY: see `CacheGuard::drop` — single-process test isolation.
    unsafe { std::env::set_var(CACHE_ENV, dir.join("project-cache")) };
    CacheGuard(prev)
}

/// Out-of-tree cache directory for `project_dir` under the pinned root.
pub fn expected_cache_dir(project_dir: &std::path::Path) -> PathBuf {
    specify_schema::cache::project_cache_dir(project_dir)
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

/// Register the JSON-body describe stub (idempotent; nextest gives each
/// test its own process, so the process-global registration is
/// per-test). The stub parses the component file's bytes as a JSON
/// `DescribeAnswer`, so a test controls its adapter's metadata by
/// writing the fixture component (`{}` for an empty answer).
pub fn register_describe_stub() {
    use specify_error::Error;
    use specify_workflow_lib::adapter::describe::{
        DescribeAnswer, DescribeRequest, register_describe_runner,
    };

    fn stub(request: &DescribeRequest<'_>) -> Result<DescribeAnswer, Error> {
        let raw = std::fs::read_to_string(request.component).map_err(|err| Error::Diag {
            code: "adapter-describe-failed",
            detail: format!("stub read {}: {err}", request.component.display()),
        })?;
        serde_json::from_str(&raw).map_err(|err| Error::Diag {
            code: "adapter-describe-failed",
            detail: format!("stub parse {}: {err}", request.component.display()),
        })
    }

    register_describe_runner(stub);
}

/// Stage a stub adapter component for `name` at the resolver's in-repo
/// development probe (`<root>/target/wasm32-wasip2/release/
/// <name>.wasm`) and register the JSON-body describe stub, so a
/// bare-name resolve inside `root` succeeds with an empty
/// `DescribeAnswer`.
pub fn stage_dev_component(root: &std::path::Path, name: &str) {
    register_describe_stub();
    let dev_dir = root.join("target/wasm32-wasip2/release");
    std::fs::create_dir_all(&dev_dir).expect("mkdir dev release dir");
    std::fs::write(dev_dir.join(format!("{}.wasm", name.replace('-', "_"))), "{}")
        .expect("write stub component");
}

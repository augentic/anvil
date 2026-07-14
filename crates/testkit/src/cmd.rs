//! [`MockCmd`], a recorder that captures every command invocation and
//! dispatches the response through a per-test closure. Pass it to
//! domain code as `&|cmd| mock.run(cmd)`.

use std::cell::RefCell;
use std::io;
use std::path::PathBuf;
use std::process::{Command, ExitStatus, Output};

/// One recorded invocation captured by [`MockCmd`].
#[derive(Debug, Clone)]
pub struct RecordedCall {
    /// The program the command would have launched.
    pub program: String,
    /// The arguments in invocation order.
    pub args: Vec<String>,
    /// The working directory set on the command, if any.
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
    /// Every recorded invocation, in call order.
    pub calls: RefCell<Vec<RecordedCall>>,
}

impl std::fmt::Debug for MockCmd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockCmd").field("calls", &self.calls).finish_non_exhaustive()
    }
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
    ///
    /// # Errors
    ///
    /// Propagates whatever the dispatch closure returns.
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
///
/// # Errors
///
/// Never fails; the `Result` wrapper matches the `CmdRunner` signature.
pub fn ok_stdout(stdout: &str) -> io::Result<Output> {
    Ok(Output {
        status: success_status(),
        stdout: stdout.as_bytes().to_vec(),
        stderr: Vec::new(),
    })
}

/// Produce a successful [`Output`] with no stdout or stderr.
///
/// # Errors
///
/// Never fails; the `Result` wrapper matches the `CmdRunner` signature.
pub fn ok_empty() -> io::Result<Output> {
    ok_stdout("")
}

/// Produce an [`Output`] whose exit status is failure with `stderr`.
///
/// # Errors
///
/// Never fails; the `Result` wrapper matches the `CmdRunner` signature.
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

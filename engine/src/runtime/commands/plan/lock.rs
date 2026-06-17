//! `specify plan lock -- <cmd>` — the command-wrapper that owns plan
//! lock acquisition. Acquires the exclusive advisory lock on
//! `<plan-root>/.specify/plan.lock`, spawns `<cmd>` under it, and
//! releases on the child's exit. The child's exit code is returned to
//! the dispatcher for passthrough; a second driver finding the lock
//! held fails fast with `plan-lock-busy`.

use std::process::Command;

use specify_error::{Error, Result};
use specify_workflow::plan_lock;

use crate::runtime::context::Ctx;

/// Environment variable the wrapper exports for the child so a nested
/// `specify plan lock` (a breakout under a parent `/spec:execute`)
/// skips re-acquisition rather than deadlocking on the lock it already
/// holds.
const HELD_ENV: &str = "SPECIFY_PLAN_LOCK_HELD";

/// Run `command` under the plan lock and return the child's exit code.
///
/// When `SPECIFY_PLAN_LOCK_HELD=1` is already set, a parent session
/// holds the lock — skip acquisition and just run the child. Otherwise
/// acquire the lock (held for the child's lifetime via the guard) and
/// export `SPECIFY_PLAN_LOCK_HELD=1` so the child's own descendants
/// re-enter without re-acquiring.
///
/// # Errors
///
/// [`Error::Argument`] when `command` is empty; the `plan-lock-busy`
/// validation error from [`plan_lock::acquire`] when another driver
/// holds the lock; [`Error::Io`] when the child cannot be spawned.
pub fn run(ctx: &Ctx, command: &[String]) -> Result<u8> {
    let (program, args) = command.split_first().ok_or_else(|| Error::Argument {
        flag: "<cmd>",
        detail: "a command to run under the plan lock is required after `--`".to_string(),
    })?;

    let already_held = std::env::var(HELD_ENV).is_ok_and(|value| value == "1");
    // Bind the guard for the whole child run so the lock is released
    // only once the child has exited. `None` on the re-entrant path —
    // the parent session still holds it.
    let _guard =
        if already_held { None } else { Some(plan_lock::acquire(ctx.layout(), ctx.now())?) };

    // Stdio is inherited by default, so the wrapped loop's output flows
    // straight through; `status()` blocks until the child exits.
    let status = Command::new(program).args(args).env(HELD_ENV, "1").status().map_err(Error::Io)?;

    // A child killed by a signal reports no code; surface it as a
    // generic non-zero failure rather than masking it as success.
    Ok(status.code().map_or(1, |code| u8::try_from(code).unwrap_or(1)))
}

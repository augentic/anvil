//! Composed-deployment host seam for the `specify` binary.
//!
//! The host layer itself is the `specify-host` binary target
//! (`src/bin/host.rs`): the macro-generated command-mode runtime over
//! the cursor-bound backends (`omnia::runtime!` — RFC-65 move 2),
//! carrying no Specify vocabulary. This library keeps the callable
//! seam the triage main consumes: [`drive`] spawns the host beside the
//! current executable (`specify-host run --config <manifest> --
//! <argv>`) with inherited stdio and verbatim exit-code passthrough.
//! The replay sibling (`specify-runtime-replay`, `src/bin/replay.rs`)
//! keeps the macro over `ModelDefault` for component-level tests and
//! examples. See `DECISIONS.md` §"One `specify` binary".

#![cfg(not(target_arch = "wasm32"))]

pub mod describe;

use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context as _, Result, bail};

/// The release-built workflow-guest component, embedded at compile time.
///
/// Sourced from the committed artifact at `crates/workflow-guest/guest.wasm`
/// (regenerate with `cargo make dist-guest` after changing guest-reachable
/// code). The triage dispatch in the `specify` binary stages these bytes
/// into its transient deployment manifest so a released binary is
/// self-contained. See `DECISIONS.md` §"Workflow-guest distribution".
pub const WORKFLOW_GUEST_WASM: &[u8] =
    include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../workflow-guest/guest.wasm"));

/// The host-layer binary name spawned by [`drive`], resolved beside the
/// current executable (same target dir in dev, same install dir in a
/// release layout).
const HOST_BINARY: &str = "specify-host";

/// Drive one guest CLI invocation through the composed deployment in
/// command mode.
///
/// Spawns the generic host layer as `specify-host run --config
/// <manifest> -- <argv>` (the runtime core prepends the deployment
/// name as `argv[0]`) with stdio inherited, so guest envelopes reach
/// the standard streams directly and the guest's exit status (low
/// byte, POSIX semantics) returns for process passthrough. Failures
/// *inside* the host — deployment assembly, backend connect
/// (`cursor-agent` missing from `PATH`) — surface on the host's own
/// stderr and pass through as its exit code; this function only errors
/// around the spawn itself.
///
/// # Errors
///
/// Returns an error when the `specify-host` binary is not found beside
/// the current executable, the spawn fails, or the host is terminated
/// by a signal (no exit code to pass through).
pub fn drive(manifest: &Path, args: Vec<String>) -> Result<u8> {
    let host = host_binary()?;
    let status = Command::new(&host)
        .arg("run")
        .arg("--config")
        .arg(manifest)
        .arg("--")
        .args(args)
        .status()
        .with_context(|| format!("spawning the host layer at {}", host.display()))?;
    match status.code() {
        Some(code) => Ok(code.to_le_bytes()[0]),
        None => bail!("the host layer terminated without an exit code ({status})"),
    }
}

/// Resolve the host-layer binary beside the current executable.
fn host_binary() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("resolving the current executable")?;
    let dir = exe.parent().context("the current executable has no parent directory")?;
    let host = dir.join(format!("{HOST_BINARY}{}", std::env::consts::EXE_SUFFIX));
    if !host.is_file() {
        bail!(
            "host layer binary `{HOST_BINARY}` not found at {} (build it with `cargo build -p \
             specify-runtime --bin {HOST_BINARY}`; releases must ship it beside `specify`)",
            host.display()
        );
    }
    Ok(host)
}

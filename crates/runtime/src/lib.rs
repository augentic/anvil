//! Composed-deployment host seam for the `specify` binary.
//!
//! The host layer is mounted in-process: [`host`] is the macro-generated
//! command-mode runtime over the cursor-bound backends (`omnia::runtime!`
//! — RFC-65 move 2), carrying no Specify vocabulary. This library keeps
//! the callable seam the provisioning front consumes: [`drive`] blocks
//! on that runtime over a deployment manifest with inherited stdio and
//! verbatim exit-code passthrough. The replay sibling
//! (`specify-runtime-replay`, `src/bin/replay.rs`) keeps the macro over
//! `ModelDefault` for component-level tests and examples. See
//! `DECISIONS.md` §"One `specify` binary".

#![cfg(not(target_arch = "wasm32"))]

pub mod describe;
pub mod host;

use std::path::Path;

use anyhow::Result;

/// Drive one guest CLI invocation through the composed deployment in
/// command mode, in-process.
///
/// Blocks on the macro-generated runtime ([`host::drive`]) over the
/// given deployment manifest, forwarding `args` verbatim (the runtime
/// core prepends the deployment name as `argv[0]`). Guest stdio is the
/// process's own, so guest envelopes reach the standard streams
/// directly, and the guest's exit status (low byte, POSIX semantics)
/// returns for process passthrough.
///
/// # Errors
///
/// Returns an error when the runtime fails ahead of the guest run —
/// deployment assembly (`building runtime: …`) or backend connect
/// (`cursor-agent` missing from `PATH`); the caller renders the anyhow
/// context chain.
pub fn drive(manifest: &Path, args: Vec<String>) -> Result<u8> {
    let builder = omnia::DeploymentBuilder::new().config(manifest.to_path_buf()).args(args);
    Ok(host::drive(builder)?.code_u8())
}

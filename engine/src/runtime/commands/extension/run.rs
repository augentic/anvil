//! `specify tool run` handler — transparent shim over the underlying
//! WASI binary that returns the guest's exit byte verbatim via
//! [`crate::runtime::output::Exit::Code`].

use specify_error::Result;
use specify_registry::host::{CapturedOutput, RunContext, WasiRunner};
use specify_registry::manifest::ExtensionScope;

use super::{build_inventory, emit_warnings_to_stderr, find};
use crate::runtime::context::Ctx;

pub fn run(ctx: &Ctx, name: &str, args: Vec<String>) -> Result<u8> {
    let inventory = build_inventory(ctx)?;
    emit_warnings_to_stderr(&inventory.warnings);
    let scoped = find(&inventory, name)?;
    let resolved = specify_registry::resolver::resolve(
        &scoped.scope,
        &scoped.tool,
        ctx.now(),
        &ctx.project_dir,
    )?;
    let mut run_ctx = RunContext::new(&ctx.project_dir, args);
    if let ExtensionScope::Plugin { capability_dir, .. } = &scoped.scope {
        run_ctx = run_ctx.with_capability_dir(capability_dir);
    }
    let runner = WasiRunner::new()?;
    let exit = runner.run(&resolved, &run_ctx)?;
    Ok(exit.clamp(0, i32::from(u8::MAX)).try_into().unwrap_or_default())
}

/// Run a declared tool with its stdout / stderr captured in memory
/// rather than inherited, returning the guest's exit code and output
/// bytes. The host-driven peer of [`run`]: used by handlers that need
/// to parse a tool's JSON envelope (e.g. `specify catalog infer
/// --phase report` folding the `vectis infer` cluster report).
///
/// # Errors
///
/// Propagates tool-not-declared, resolver, and Wasmtime runtime errors
/// from the same boundary as [`run`].
pub fn run_captured(ctx: &Ctx, name: &str, args: Vec<String>) -> Result<CapturedOutput> {
    let inventory = build_inventory(ctx)?;
    emit_warnings_to_stderr(&inventory.warnings);
    let scoped = find(&inventory, name)?;
    let resolved = specify_registry::resolver::resolve(
        &scoped.scope,
        &scoped.tool,
        ctx.now(),
        &ctx.project_dir,
    )?;
    let mut run_ctx = RunContext::new(&ctx.project_dir, args);
    if let ExtensionScope::Plugin { capability_dir, .. } = &scoped.scope {
        run_ctx = run_ctx.with_capability_dir(capability_dir);
    }
    let runner = WasiRunner::new()?;
    Ok(runner.run_captured(&resolved, &run_ctx)?)
}

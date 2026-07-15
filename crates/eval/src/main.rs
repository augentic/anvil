//! Prompt-evaluation harness: one live-model trial that drives the
//! Specify engine the same way an operator does.
//!
//! ```text
//! init        scaffold the fixture-bound project
//! plan        author the change, stamp Gate 1 (`approved`)
//! execute     drain the loop: refine → build → merge per slice
//! finalize    archive the drained plan
//! ```
//!
//! Graded by deterministic validators only (see [README.md](../README.md)).
//! Run `cargo make eval` (never CI). Needs `cursor-agent` on
//! `PATH` with credentials. Runs in `sandbox/eval/`; the project is
//! removed on success and retained on failure.

#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
mod telemetry;
#[cfg(not(target_arch = "wasm32"))]
mod trial;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, Parser)]
#[command(about = "Run the live-model prompt evaluation")]
struct Cli {
    #[command(subcommand)]
    phase: Option<trial::Phase>,
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    match Cli::parse().phase {
        Some(phase) => trial::run_phase(phase).await,
        None => trial::run().await,
    }
}

// The harness is native-only; the stub keeps workspace-wide wasm32
// builds green without pulling the cursor backend into the guest graph.
#[cfg(target_arch = "wasm32")]
fn main() {}

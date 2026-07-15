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
//! `PATH` with credentials. The temporary project is retained on failure.

#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
mod telemetry;
#[cfg(not(target_arch = "wasm32"))]
mod trial;

#[cfg(not(target_arch = "wasm32"))]
#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    trial::run().await;
}

// The harness is native-only; the stub keeps workspace-wide wasm32
// builds green without pulling the cursor backend into the guest graph.
#[cfg(target_arch = "wasm32")]
fn main() {}

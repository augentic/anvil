//! # Prompt-evaluation harness
//!
//! A live-model trial that drives the Specify engine the same way an operator does.
//!
//! ```text
//! init        scaffold the fixture-bound project
//! plan        author the change, stamp Gate 1 (`approved`)
//! execute     drain the loop: refine → build → merge per slice
//! finalize    archive the drained plan
//! ```
//!
//! See [README.md](../README.md) for more details.

mod grade;
mod native;
mod telemetry;
mod trial;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(about = "Run the live-model prompt evaluation")]
struct Cli {
    #[command(subcommand)]
    phase: Option<trial::Phase>,
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    match Cli::parse().phase {
        Some(phase) => phase.run().await,
        None => trial::run().await,
    }
}

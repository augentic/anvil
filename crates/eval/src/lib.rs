//! Lab-only live-model evaluation over the linked host.
//!
//! `eval` owns the multi-step live workflow trial, single-operation
//! adapter prompt scenarios, deterministic grading, model-request
//! telemetry, sandbox seeding and cleanup, and the eval CLI parsing.
//! It constructs no concrete adapter catalog, no Cursor backend, and
//! no Tokio runtime: the process-facing client ([`run`]) receives a
//! workspace root, a validated [`linked::Catalog`], and a
//! [`ModelFactory`] from its composition root (a repository `lab`
//! binary), and drives workflow phases through the linked command API.

mod fs;
pub mod grade;
mod run;
pub mod sandbox;
pub mod scenario;
pub mod telemetry;

pub use run::{ModelFactory, ModelInstance, run};

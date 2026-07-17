//! Shared guest support for Specify adapter components.
//!
//! Per-adapter crates implement [`Source`] or [`Target`] on a unit type;
//! the wasm export macros and native harnesses consume that trait. The
//! rest of this crate is the vocabulary and helpers those implementors
//! share: seam DTOs, judgment/`repaired` calls, embedded prose registry,
//! and the MCP references server.

pub mod answers;
mod call;
mod operations;
pub mod phase;
pub mod references;
pub mod registry;
pub mod seam;

#[cfg(target_arch = "wasm32")]
pub mod source;
#[cfg(target_arch = "wasm32")]
pub mod target;

pub use call::{MAX_REPAIRS, judgment, repaired};
pub use omnia_guest::Model;
#[cfg(target_arch = "wasm32")]
pub use omnia_guest::model::WasiModel;
pub use omnia_guest::model::{
    Error, Format, McpGrant, Message, Reply, Request, Role, SchemaFormat, Tool,
};
pub use operations::{Source, Target};
/// Re-exported for the `source!` / `target!` macro expansions.
#[cfg(target_arch = "wasm32")]
pub use wasip3;

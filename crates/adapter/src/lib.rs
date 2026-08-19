//! Shared guest support for Emery adapter components.
//!
//! Per-adapter crates implement [`Source`] on a unit type; the wasm
//! export macro and the native host consume that trait.

pub mod answers;
mod call;
mod identity;
mod operations;
pub mod references;
pub mod registry;
pub mod seam;

#[cfg(target_arch = "wasm32")]
pub mod source;

pub use call::{MAX_REPAIRS, judgment, repaired};
pub use identity::AdapterIdentity;
pub use omnia_guest::Model;
#[cfg(target_arch = "wasm32")]
pub use omnia_guest::model::WasiModel;
pub use omnia_guest::model::{
    Error, Format, McpGrant, Message, Reply, Request, Role, SchemaFormat, Tool,
};
pub use operations::Source;
/// Re-exported for the `source!` / `target!` macro expansions.
#[cfg(target_arch = "wasm32")]
pub use wasip3;

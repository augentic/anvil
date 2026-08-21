//! Shared guest support for Emery adapter components.
//!
//! Per-adapter crates implement [`SourceAdapter`] on a unit type; the
//! wasm export macro consumes that trait. The engine provider
//! implements [`Source`] — the import-side capability, like
//! [`omnia_guest::Model`].

pub mod answers;
mod call;
pub mod dispatch;
mod identity;
mod operations;
pub mod references;
pub mod registry;
pub mod seam;

#[cfg(target_arch = "wasm32")]
pub mod source;

pub use call::{MAX_REPAIRS, judgment, repaired};
pub use dispatch::{DispatchError, Source};
pub use identity::{AdapterIdentity, IdentityError};
pub use omnia_guest::Model;
#[cfg(target_arch = "wasm32")]
pub use omnia_guest::model::WasiModel;
pub use omnia_guest::model::{
    Error, Format, McpGrant, Message, Reply, Request, Role, SchemaFormat, Tool,
};
pub use operations::SourceAdapter;
/// Re-exported for the `source!` / `target!` macro expansions.
#[cfg(target_arch = "wasm32")]
pub use wasip3;

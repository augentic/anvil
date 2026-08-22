//! SDK for Emery source adapter components.
//!
//! Adapters implement [`SourceAdapter`]; engine providers implement the
//! import-side [`Source`] capability.

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
/// Runtime bindings used by [`source!`].
#[cfg(target_arch = "wasm32")]
pub use wasip3;

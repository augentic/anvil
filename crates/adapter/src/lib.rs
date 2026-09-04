//! SDK for Emery source adapter components.
//!
//! Adapters implement [`SourceAdapter`]. The `emery:adapter/source`
//! contract itself (the WIT DTOs, the import-side [`Source`] capability)
//! is `emery-source`, re-exported here so an adapter needs one crate.

pub mod answers;
mod call;
mod operations;
pub mod references;
pub mod types;

#[cfg(target_arch = "wasm32")]
pub mod source;

pub use call::{MAX_REPAIRS, judgment, repaired};
pub use emery_source::{AdapterIdentity, DispatchError, IdentityError, SOURCE_INTERFACE, Source};
pub use omnia_guest::Model;
#[cfg(target_arch = "wasm32")]
pub use omnia_guest::model::WasiModel;
pub use omnia_guest::model::{
    Error, Format, Function, Message, Reply, Request, Role, SchemaFormat, Tool, ToolCall,
};
pub use operations::SourceAdapter;

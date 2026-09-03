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
pub mod types;

#[cfg(target_arch = "wasm32")]
pub mod source;

/// The versioned `source` interface a deployment declares as its plugin
/// seam; must track the `emery:adapter` WIT package version.
pub const SOURCE_INTERFACE: &str = "emery:adapter/source@0.1.0";

pub use call::{MAX_REPAIRS, judgment, repaired};
pub use dispatch::{DispatchError, Source};
pub use identity::{AdapterIdentity, IdentityError};
pub use omnia_guest::Model;
#[cfg(target_arch = "wasm32")]
pub use omnia_guest::model::WasiModel;
pub use omnia_guest::model::{
    Error, Format, Function, Message, Reply, Request, Role, SchemaFormat, Tool, ToolCall,
};
pub use operations::SourceAdapter;

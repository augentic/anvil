//! Source adapter SDK
//!
//! Everything an adapter author needs to build an Emery source adapter: the
//! [`SourceAdapter`] trait to implement, the model judgment helpers, the
//! evidence answer schema, and the export macro that turns an implementation
//! into a wasm component.
//!
//! The contract itself lives in `emery-source` and is re-exported here, so an
//! adapter depends on one crate and never sees the wire bindings directly.

pub mod answers;
mod call;
mod operations;
pub mod references;
pub mod types;

#[cfg(target_arch = "wasm32")]
pub mod source;

pub use call::{MAX_REPAIRS, judgment, repaired};
pub use emery_source::{DispatchError, SOURCE_INTERFACE, Source};
pub use omnia_guest::Model;
#[cfg(target_arch = "wasm32")]
pub use omnia_guest::model::WasiModel;
pub use omnia_guest::model::{
    Error, Format, Function, Message, Reply, Request, Role, SchemaFormat, Tool, ToolCall,
};
pub use operations::SourceAdapter;

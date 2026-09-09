//! Source adapter SDK
//!
//! Everything an adapter author needs to build an Emery source adapter: the
//! [`SourceAdapter`] trait to implement, the [`evidence`] call that asks the
//! extract question, the [`content_note`] prompt fragment, and the export
//! macro that turns an implementation into a wasm component.
//!
//! The contract itself lives in `emery-source` and is re-exported here, so an
//! adapter depends on one crate and never sees the wire bindings directly.

mod answers;
mod operations;
pub mod references;
pub mod types;

#[cfg(target_arch = "wasm32")]
pub mod source;

pub use answers::{content_note, evidence};
pub use emery_source::{DispatchError, SOURCE_INTERFACE, Source};
pub use omnia_guest::Model;
#[cfg(target_arch = "wasm32")]
pub use omnia_guest::model::WasiModel;
pub use omnia_guest::model::{
    Error, Findings, Format, Function, Message, Question, Reply, Request, Role, SchemaFormat, Tool,
    ToolCall, ToolFuture, Tools,
};
pub use operations::SourceAdapter;

//! Embedded prompt corpora: the runtime registry over a build-time `DOCS`
//! table, and (feature `emit`) the codegen that writes it.

pub mod registry;

#[cfg(feature = "emit")]
mod emit;

#[cfg(feature = "emit")]
pub use emit::emit;

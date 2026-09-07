//! Embedded prompt corpora
//!
//! Prompts and reference documents ship inside the binaries that use them —
//! the engine's synthesis prose and each adapter's extraction prose — rather
//! than being read from disk at run time. This crate is the shared way to do
//! that: a build-time step that embeds a Markdown tree, and a small runtime
//! [`mod@registry`] for looking documents up by path.
//!
//! The build-time half lives behind the `emit` feature so it is only pulled
//! into build scripts, never into a shipped guest.

pub mod registry;

#[cfg(feature = "emit")]
mod emit;

#[cfg(feature = "emit")]
pub use emit::emit;

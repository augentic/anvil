//! Clap action-enum surface for the native-only `specify adapters`
//! verb family.
//!
//! The handlers live in the binary crate (hydration needs the
//! wasm-pkg transport and the global store); only the grammar is
//! carried here so the full `Commands` tree parses everywhere.

pub mod cli;

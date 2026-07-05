//! Clap action-enum surface for the native-only `specify catalog` verb
//! family. The handlers live in the binary crate; only the grammar is
//! carried here so the full `Commands` tree parses everywhere.

pub mod cli;

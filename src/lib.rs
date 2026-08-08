//! The emery guest: the deployment's only `wasi:cli/run` exporter.
//! One macro invocation over the `guest` crate; `build.rs` embeds the
//! built component into the shipped binary as `$OUT_DIR/emery.bin`.
#![cfg(target_arch = "wasm32")]

guest::export!();

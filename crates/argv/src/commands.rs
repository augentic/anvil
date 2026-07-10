//! Per-family clap grammar plus the clap-to-`Input` conversions.
//!
//! Each family module carries its clap action enum under `cli` (so
//! the full [`crate::cli::Commands`] tree parses everywhere) and,
//! where the wire shapes differ, the conversions from clap value
//! types to the matching handler `Input` DTOs. The handler bodies live in
//! `workflow`'s domain modules (each family in a `handlers` submodule);
//! the exhaustive dispatch matches live in the shims.

pub mod adapters;
pub mod archive;
pub mod journal;
pub mod plan;
pub mod plugins;
pub mod registry;
pub mod slice;
pub mod source;
pub mod target;
pub mod workspace;

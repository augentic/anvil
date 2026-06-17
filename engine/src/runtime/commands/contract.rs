//! `specify contract dump` — the machine-readable CLI contract.
//!
//! `dump` walks the live clap tree and the closed const tables (exit
//! codes, error discriminants, journal event ids, embedded schema
//! paths) into one `CliContract` payload, so documentation and skill
//! prose can be cross-checked against the binary instead of by `rg`
//! discipline.

pub mod cli;
pub mod dump;

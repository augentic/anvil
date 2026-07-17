//! `eval` — the canonical fixture registry linked into the shared
//! harness, plus the trial sandbox locator.

pub use fixture::{Fixtures, catalog};

/// The sandbox root for the trial project.
pub const SANDBOX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../sandbox");

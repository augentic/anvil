//! `eval` — the canonical fixture registry linked into the shared
//! harness, plus the trial sandbox locator.

use harness::catalog::{Binding, Catalog};
use omnia_guest::Model;

/// Every fixture adapter linked into `eval`.
#[must_use]
pub fn catalog<M: Model>() -> Catalog<M> {
    fixture::catalog()
}

/// The adapter binding handed to the shared harness entrypoints.
#[derive(Clone, Copy, Debug)]
pub struct Adapters;

impl Binding for Adapters {
    fn catalog<M: Model>() -> Catalog<M> {
        catalog()
    }
}

/// The sandbox root for the trial project.
pub const SANDBOX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../sandbox");

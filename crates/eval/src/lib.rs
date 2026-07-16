//! `eval` — the testkit fixtures linked into the shared harness, plus
//! the trial sandbox locator.

use harness::catalog::{Binding, Catalog};
use omnia_guest::Model;
use testkit::fixture::{Fixture, FixtureCode, FixtureDocs};

/// The Specify-owned fixture adapters linked into the engine trial.
#[must_use]
pub fn catalog<M: Model>() -> Catalog<M> {
    Catalog::builder()
        .source::<Fixture>()
        .source::<FixtureDocs>()
        .source::<FixtureCode>()
        .target::<Fixture>()
        .build()
}

/// The fixture binding handed to the shared harness entrypoints.
#[derive(Clone, Copy, Debug)]
pub struct Fixtures;

impl Binding for Fixtures {
    fn catalog<M: Model>() -> Catalog<M> {
        catalog()
    }
}

/// The sandbox root for the trial project.
pub const SANDBOX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../sandbox");

//! The exhaustive fixture registry linked into the shared harness.

use harness::catalog::Catalog;
use omnia_guest::Model;

use crate::ops::{
    FailBuild, FailExtract, FailGuidance, FailMerge, FailSurvey, Fixture, FixtureCode, FixtureDocs,
    MissingOutput,
};

/// Every fixture catalog identity, success and failure profiles alike.
///
/// Axis correctness is compile-checked by the typed `source::<A>` /
/// `target::<A>` registrations; the registry integration test asserts
/// the exact `(axis, name)` inventory against this declaration.
#[must_use]
pub fn catalog<M: Model>() -> Catalog<M> {
    Catalog::builder()
        .source::<Fixture>()
        .source::<FixtureDocs>()
        .source::<FixtureCode>()
        .source::<FailSurvey>()
        .source::<FailExtract>()
        .target::<Fixture>()
        .target::<FailGuidance>()
        .target::<FailBuild>()
        .target::<FailMerge>()
        .target::<MissingOutput>()
        .build()
}

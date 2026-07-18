//! The exhaustive fixture registry linked into the workflow suites.

use linked::Catalog;

use crate::ops::{
    Adapter, Code, Docs, FailBuild, FailExtract, FailGuidance, FailMerge, FailSurvey, MissingOutput,
};

/// Every fixture catalog identity, success and failure profiles alike.
///
/// Axis correctness is compile-checked by the typed `source::<A>` /
/// `target::<A>` registrations; the registry integration test asserts
/// the exact `(axis, name)` inventory against this declaration.
///
/// # Panics
///
/// Never in practice: the fixture inventory is statically valid, so
/// catalog validation cannot fail.
#[must_use]
pub fn catalog() -> Catalog {
    Catalog::builder()
        .source::<Adapter>()
        .source::<Docs>()
        .source::<Code>()
        .source::<FailSurvey>()
        .source::<FailExtract>()
        .target::<Adapter>()
        .target::<FailGuidance>()
        .target::<FailBuild>()
        .target::<FailMerge>()
        .target::<MissingOutput>()
        .build()
        .expect("the fixture catalog is statically valid")
}

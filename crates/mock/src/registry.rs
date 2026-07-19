//! The exhaustive mock registry linked into the engine suites.

use native::Catalog;

use crate::ops::{
    Adapter, Code, Docs, FailBuild, FailExtract, FailGuidance, FailMerge, FailSurvey, MissingOutput,
};

/// Every mock catalog identity, success and failure profiles alike.
///
/// Axis correctness is compile-checked by the typed `source::<A>` /
/// `target::<A>` registrations; the registry integration test asserts
/// the exact `(axis, name)` inventory against this declaration.
///
/// # Panics
///
/// Never in practice: the mock inventory is statically valid, so
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
        .expect("the mock catalog is statically valid")
}

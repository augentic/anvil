//! The exhaustive fixture registry linked into the shared harness.

use harness::catalog::Catalog;
use omnia_guest::Model;

use crate::ops::{
    Adapter, Code, Docs, FailBuild, FailExtract, FailGuidance, FailMerge, FailSurvey, MissingOutput,
};

/// Every fixture catalog identity, success and failure profiles alike.
///
/// Axis correctness is compile-checked by the typed `source::<A>` /
/// `target::<A>` registrations; the registry integration test asserts
/// the exact `(axis, name)` inventory against this declaration.
#[must_use]
pub fn catalog<M: Model>() -> Catalog<M> {
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
}

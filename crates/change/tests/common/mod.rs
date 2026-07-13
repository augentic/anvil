//! Shared test helpers for the `change` integration tests: the
//! project-level base helpers (`MockCmd`, `Project`, plan builders)
//! shared by `#[path]` from the `project` crate, plus the change-loop
//! extras — scripted judgment answers (`answers`) and the fixture
//! adapter provider (`fixture`).

#![expect(
    dead_code,
    reason = "shared test helpers; not every integration binary uses every helper"
)]

pub mod answers;
pub mod fixture;

#[path = "../../../project/tests/common/mod.rs"]
mod base;

pub use base::*;

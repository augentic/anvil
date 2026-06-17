//! Integration tests for the `specify slice` subcommand tree.
//!
//! Every test stands up a fresh `.specify/` project via `specify init`,
//! drives `specify slice *` through `assert_cmd`, and inspects both the
//! structured stdout (`--format json`) and the on-disk side effects the
//! verb is responsible for.
//!
//! Test style follows `tests/e2e.rs`: favour end-to-end execution of the
//! built binary over unit tests so the behaviour the skills consume is
//! the behaviour under test.
//!
//! The suite is split across themed submodules under `tests/slice/`;
//! shared imports, helpers, and seeds live in [`support`].

mod common;

#[path = "slice/support.rs"]
mod support;

#[path = "slice/create.rs"]
mod create;

#[path = "slice/transition.rs"]
mod transition;

#[path = "slice/touched_specs.rs"]
mod touched_specs;

#[path = "slice/overlap.rs"]
mod overlap;

#[path = "slice/drop.rs"]
mod drop;

#[path = "slice/metadata.rs"]
mod metadata;

#[path = "slice/validate.rs"]
mod validate;

#[path = "slice/provenance.rs"]
mod provenance;

#[path = "slice/model_show.rs"]
mod model_show;

#[path = "slice/validate_file_location.rs"]
mod validate_file_location;

#[path = "slice/validate_catalog.rs"]
mod validate_catalog;

#[path = "slice/synthesize.rs"]
mod synthesize;

#[path = "slice/plan_dir.rs"]
mod plan_dir;

#[path = "slice/build.rs"]
mod build;

#[path = "slice/decisions.rs"]
mod decisions;

#[path = "slice/drift.rs"]
mod drift;

#[path = "slice/merge.rs"]
mod merge;

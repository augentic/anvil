//! Plan CLI surface: the `emery plan *` operations (`handlers`) and
//! their wire DTOs (`wire`). The `plan.yaml` state machine and the
//! `plan validate` health diagnostics live in [`project::plan`].

pub mod handlers;
pub mod wire;

//! Integration tests for the `specify plan *` surface: the CLI
//! orchestration verbs and the fan-in/fan-out reconciliation. CLI
//! submodules live under `workflow/`; the fan-in/out suite lives under
//! `plan/`. Pure plan-schema tests live in
//! `crates/workflow/tests/plan_schema.rs`. Shared helpers live in
//! [`common`]; the orchestration submodules pull their shared surface
//! in via [`support`].

mod common;

#[path = "workflow/support.rs"]
mod support;

#[path = "plan/end_to_end.rs"]
mod end_to_end;

#[path = "workflow/validate.rs"]
mod validate;

#[path = "workflow/next.rs"]
mod next;

#[path = "workflow/status.rs"]
mod status;

#[path = "workflow/mutate.rs"]
mod mutate;

#[path = "workflow/source_binding.rs"]
mod source_binding;

#[path = "workflow/transition.rs"]
mod transition;

#[path = "workflow/plan_lock.rs"]
mod plan_lock;

#[path = "workflow/create.rs"]
mod create;

#[path = "workflow/archive.rs"]
mod archive;

#[path = "workflow/authority.rs"]
mod authority;

#[path = "workflow/propose.rs"]
mod propose;

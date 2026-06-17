//! Consolidated integration binary for `specify-standards`.
//!
//! One binary per crate: each former `tests/<area>.rs` hub is pulled in here as
//! a `#[path]` submodule so the crate-under-test links exactly once. The shared
//! `common` and `eval_support` helpers are declared a single time and every area
//! reaches them as `crate::common` / `crate::eval_support`. See
//! [docs/standards/testing.md](../../../docs/standards/testing.md).

mod common;
mod eval_support;

#[path = "lint_engine_guards.rs"]
mod lint_engine_guards;
#[path = "lint_hint.rs"]
mod lint_hint;
#[path = "lint_index.rs"]
mod lint_index;

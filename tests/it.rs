//! Consolidated integration binary for the `specify` crate.
//!
//! One binary per crate: each former `tests/<area>.rs` is pulled in here as a
//! `#[path]` submodule so the crate-under-test links exactly once instead of
//! once per area. The shared `common` helper is declared a single time and
//! every area reaches it as `crate::common`. `rust_quality` stays a separate
//! dev-gate binary. See [docs/standards/testing.md](../docs/standards/testing.md).

mod common;

#[path = "archive.rs"]
mod archive;
#[path = "bootstrap.rs"]
mod bootstrap;
#[path = "cli.rs"]
mod cli;
#[path = "dist.rs"]
mod dist;
#[path = "e2e.rs"]
mod e2e;
#[path = "guest.rs"]
mod guest;
#[path = "init.rs"]
mod init;
#[path = "journal.rs"]
mod journal;
#[path = "lint.rs"]
mod lint;
#[path = "plan.rs"]
mod plan;
#[path = "registry.rs"]
mod registry;
#[path = "rules.rs"]
mod rules;
#[path = "slice.rs"]
mod slice;
#[path = "source.rs"]
mod source;
#[path = "target.rs"]
mod target;
#[path = "workspace.rs"]
mod workspace;

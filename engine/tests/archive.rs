//! Integration tests for `specify archive *` — the retention GC over
//! `.specify/archive/`. Shared helpers live in [`common`].

mod common;

#[path = "archive/prune.rs"]
mod prune;

//! Integration tests for the `specify rules` surface (`export` and the
//! shared codex distribution). Shared helpers live in [`common`].

mod common;

#[path = "rules/export.rs"]
mod export;

#[path = "rules/codex.rs"]
mod codex;

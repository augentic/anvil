//! Integration tests for the top-level CLI surface (dispatch + error
//! mapping). Shared helpers live in [`common`].

mod common;

#[path = "cli/base.rs"]
mod base;

#[path = "cli/errors.rs"]
mod errors;

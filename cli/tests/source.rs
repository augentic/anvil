//! Integration tests for the `specify source` subcommand tree
//! (`resolve`, `extract`, `preview`, `survey`). Shared helpers live in
//! [`common`].

mod common;

#[path = "source/resolve.rs"]
mod resolve;

#[path = "source/extract.rs"]
mod extract;

#[path = "source/preview.rs"]
mod preview;

#[path = "source/survey.rs"]
mod survey;

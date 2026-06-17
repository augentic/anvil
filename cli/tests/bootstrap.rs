//! Integration tests for the bootstrap lifecycle verbs
//! (`upgrade`, `plugins`). Shared helpers live in [`common`].

mod common;

#[path = "bootstrap/upgrade.rs"]
mod upgrade;

#[path = "bootstrap/plugins.rs"]
mod plugins;

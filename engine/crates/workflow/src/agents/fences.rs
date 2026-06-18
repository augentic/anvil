//! Byte-preserving parser and write planner for the fenced `AGENTS.md`
//! context block. [`parse_document`] decodes a candidate document;
//! [`plan_agents_write`] composes the final write plan that preserves
//! operator-authored bytes.

mod parse;
mod render;

pub use parse::{FenceError, parse_document};
pub use render::{WriteDisposition, plan_agents_write};

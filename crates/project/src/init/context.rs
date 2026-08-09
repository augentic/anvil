//! Init-time `AGENTS.md` context generation.
//!
//! Thin façade over [`crate::agents::generate`]: skip when `AGENTS.md`
//! already exists.

pub(super) use crate::agents::{Skip, generate};

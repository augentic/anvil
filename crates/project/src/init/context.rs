//! Init-time `AGENTS.md` context generation.
//!
//! Thin façade over [`crate::agents::generate`]: skip when `AGENTS.md`
//! already exists or the project is a materialised workspace slot.

pub(super) use crate::agents::{Skip, generate};

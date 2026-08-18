//! Artifact types and parsers plus the shared atomic writer.
//!
//! A lifecycle-free leaf: a parser cannot transition engine state.

pub mod atomic;
pub mod evidence;
pub mod spec;

//! Artifact types and parsers, the shared atomic writer, and the
//! validation rule registry ([`validate`]).
//!
//! A lifecycle-free leaf: a rule cannot transition a slice or stamp a plan.

pub mod atomic;
pub mod decision;
pub mod discovery;
pub mod evidence;
pub mod spec;
pub mod task;
pub mod validate;

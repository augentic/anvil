//! Slice synthesis projection kernel.
//!
//! The agent owns cross-modal reconciliation; the kernel is a pure,
//! IO-free projection over the agent's returned structure.

pub mod authority;
pub mod baseline;
pub mod evidence;
pub mod persist;
pub mod project;
pub mod render;
pub mod stage;
pub mod wire;

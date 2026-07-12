//! Library face of the live quality orchestrator.
//!
//! The binary owns argument parsing and the per-profile drivers; the
//! reusable seams live here: the in-process composed [`executor`], the
//! typed deployment [`manifest`] builder, the process-spawning
//! [`verify`] evaluator, and the [`trial`] grading loop a
//! credential-free test can exercise with a fake
//! [`scenario::evaluate::semantic::Judge`].

pub mod executor;
pub mod judge;
pub mod manifest;
pub mod trial;
pub mod verify;

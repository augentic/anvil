//! Pure profile-specific evaluators over trial workspaces.
//!
//! [`crate::grade::hard_with`] settles `kind: registered` probes
//! through a profile-built [`crate::grade::Evaluators`] registry. The
//! filesystem-only evaluators ship here — the guest-loop journal
//! cadence ([`crate::evaluate::guest`]) and the composed-loop artifact
//! checks ([`crate::evaluate::composed`]) — so every harness registers
//! the same implementations. Evaluators that spawn processes (for
//! example generated-crate verification via `cargo check`) belong to
//! the owning harness; this crate spawns nothing. Semantic rubric
//! grading lives in [`crate::evaluate::semantic`] behind the same
//! boundary.

pub mod composed;
pub mod guest;
pub mod semantic;

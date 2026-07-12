//! Profile-specific evaluators over trial workspaces.
//!
//! [`crate::grade::hard`] settles every generic probe; the modules here
//! own the two grading legs a generic probe cannot express: the
//! registered guest-loop assertions ([`crate::evaluate::guest`]) and
//! live semantic rubric grading ([`crate::evaluate::semantic`]).

pub mod guest;
pub mod semantic;

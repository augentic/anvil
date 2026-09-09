//! Contract types
//!
//! The Rust forms of the records in the `emery:adapter` WIT package — the
//! input an adapter receives and the [`Evidence`] it returns. These are the
//! types engine and adapter code work with; the generated wire bindings stay
//! behind them. Operations fail with `omnia_guest::Error`; the WIT error
//! variant is lowered and lifted inside the wire module alone.
//!
//! Serde derives sit only on the types that cross a JSON boundary: the
//! [`Evidence`] a model answer is parsed into, and the [`SourceInput`] shape
//! the engine's `specify` request carries.

mod source;

pub use source::{
    Authority, Backing, Claim, ClaimKind, Evidence, SourceContent, SourceInput, SourceMetadata,
};

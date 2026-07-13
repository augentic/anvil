//! Target build envelope kernel.
//!
//! Mirrors [`crate::synthesis`]: the pure, IO-free domain pieces
//! the `specify slice build` verb composes. The closed-shape build
//! request/report DTOs live in [`project::seam::wire`] (the typed
//! serde parse is the envelope gate); [`assemble`]
//! assembles a request from the bound target adapter's declared inputs
//! against the slice tree.

pub mod assemble;

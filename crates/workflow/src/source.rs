//! Source-axis command surface.
//!
//! The `specify source survey` / `specify source extract` operations
//! live in [`handlers`], each driving the matching
//! [`crate::orchestrate`] kernel through the provider's
//! [`crate::seam::SourceSeam`].

pub mod handlers;

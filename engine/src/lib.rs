//! Specify CLI library — hosts the `specify` dispatch tree.

mod output;
pub mod runtime;

pub use output::Format;
pub use runtime::run;

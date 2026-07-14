//! Model doubles: re-exports of `omnia-testkit`'s recorded harness and
//! FIFO script. Prompt pinning lives in [`crate::goldens`], asserted
//! over the harness's recorded requests.

pub use omnia_testkit::model::{Harness, Scripted};

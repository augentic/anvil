//! Password reset slice — Omnia crate skeleton.
//!
//! This is the expected output of `targets/omnia/briefs/build.md` § Crate writer
//! when run against `tests/fixtures/targets/omnia/input/`. It is a skeleton: handler
//! bodies are stubbed but the structural shape — module layout, error enum, handler
//! delegation, provider trait bounds — is the contract the build brief must produce.

pub mod error;
pub mod handlers;

pub use error::ResetError;
pub use handlers::{ResetAck, ResetEvent, ResetRequest};

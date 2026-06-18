//! `specify workspace *` handlers — `sync`, `prepare`, `push`.

pub mod cli;

mod prepare;
mod push;
mod sync;

pub use prepare::prepare;
pub use push::push;
use specify_error::Error;
pub use sync::sync;

pub(super) fn registry_missing() -> Error {
    Error::Diag {
        code: "registry-missing",
        detail: "no registry declared at registry.yaml".to_string(),
    }
}

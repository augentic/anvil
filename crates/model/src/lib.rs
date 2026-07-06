//! Specify model — artifact types and parsers (`spec`, `task`,
//! `evidence`, `discovery`) plus the shared atomic writer and the
//! artifact validation rule registry ([`validate`]). A lifecycle-free
//! leaf beneath `specify-workflow`: nothing here can transition a slice
//! or stamp a plan, so a validation rule physically cannot reach a
//! lifecycle write. See `docs/standards/architecture.md` for the
//! rationale.

pub mod atomic;
pub mod decision;
pub mod discovery;
pub mod evidence;
pub mod spec;
pub mod task;
pub mod validate;

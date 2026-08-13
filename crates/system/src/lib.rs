//! The RFC-104 definition loop: definition-home layout, declared
//! boundary and coverage DTOs, and the `emery system *` kernels.
//! The definition home is durable client architecture, not a product root.

pub mod coverage;
pub mod handlers;
pub mod layout;
pub mod materialize;
pub mod orchestrate;
pub mod scope;

pub use coverage::{Coverage, Disposition, Row, RowPatch, SurveyError, SurveyErrorKind};
pub use layout::Layout;
pub use scope::Scope;

/// Ceiling on the summed survey lead count, checked before any `extract`.
///
/// Counts every included source that completed `survey` this run.
/// Exceeding it is the typed stop `system-survey-lead-limit`; recovery
/// is D2 — narrow coverage or author another definition home. An
/// engine constant beside `project::judgment::MAX_REPAIRS`, never
/// operator-configurable.
pub const MAX_SURVEY_LEADS: usize = 256;

/// Ceiling on the included Evidence set's claim count.
///
/// Checked before the correlation judgment. Exceeding it is the typed
/// stop `system-correlation-claim-limit`; recovery is D2. An engine
/// constant, never operator-configurable.
pub const MAX_CORRELATION_CLAIMS: usize = 4096;

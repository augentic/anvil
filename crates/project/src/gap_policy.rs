//! Closed gap-policy enum — a project's declared build-gate posture
//! toward open gap rows (RFC-86a D3).

use serde::{Deserialize, Serialize};

/// Build-gate posture toward open gap rows (`[unknown]` / `[conflict]`).
///
/// Declared optionally on `project.yaml` (`gap-policy:`) via
/// `emery init --gap-policy` and preserved by `init --upgrade`; an
/// absent declaration means [`Strict`]. The declaration is a standing
/// project posture — the execute loop resolves the effective policy
/// per epoch (`--gap-policy` flag, else this declaration, else
/// `strict`).
///
/// [`Strict`]: GapPolicy::Strict
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Serialize,
    Deserialize,
    strum::Display,
    strum::EnumString,
)]
#[serde(rename_all = "kebab-case")]
#[strum(serialize_all = "kebab-case")]
pub enum GapPolicy {
    /// Open gap rows block build (the built-in default).
    #[default]
    Strict,
    /// Open gap rows are dispositioned as `origin: policy` deferrals
    /// at the build gate and build proceeds.
    Defer,
}

//! Shared probe implementors for the linked-host test binaries.
//!
//! One dual-axis [`Probe`] registered as `fixture`, the always-failing
//! [`FailGuidance`] target, the [`Floored`] target carrying a
//! metadata floor, the [`Reflect`] source echoing its reference URL,
//! and the [`BadVersion`] target with a non-SemVer identity — enough
//! surface for the catalog, provider, references, and command suites
//! without a dependency on any concrete adapter crate (the checks
//! package pins that invariant).

#![allow(dead_code, reason = "each test binary uses a subset of the shared support surface")]

use adapter::registry::Doc;
use adapter::seam::{
    BuildOutput, Context, Error, Evidence, Input, Lead, MergePhase, Platform, Report,
    SourceMetadata, Status, TargetMetadata, WorkingTree,
};
use adapter::{AdapterIdentity, Source, Target};
use omnia_guest::Model;
use omnia_guest::model::{Format, Request};

/// The single embedded reference document every probe serves.
pub const DOCS: &[Doc] = &[Doc {
    path: "prompts/guidance.md",
    body: "fixture guidance",
}];

/// Dual-axis probe implementor, `fixture` on both axes.
///
/// Every leg echoes its inputs so the suites can assert dispatch
/// threading: `survey` returns the model's answer as the lead and the
/// routed id in the synopsis, `build` and `merge` record their
/// arguments in the report's single output path, and `extract` fails
/// with a typed error naming the lead.
#[derive(Clone, Copy, Debug)]
pub struct Probe;

impl Source for Probe {
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "fixture",
        version: "0.0.0",
    };

    fn metadata() -> SourceMetadata {
        SourceMetadata { specify_floor: None }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn survey<P: Model>(model: &P, ctx: &Context<'_>) -> Result<Vec<Lead>, Error> {
        let reply = model
            .create(Request {
                format: Format::Json,
                ..Request::default()
            })
            .await
            .map_err(Error::from)?;
        Ok(vec![Lead {
            lead: reply.answer,
            synopsis: format!("surveyed by {}", ctx.adapter_id),
            topics: Vec::new(),
        }])
    }

    async fn extract<P: Model>(
        _model: &P, _ctx: &Context<'_>, lead: &Lead,
    ) -> Result<Evidence, Error> {
        Err(Error::Internal(format!("no evidence for {}", lead.lead)))
    }
}

impl Target for Probe {
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "fixture",
        version: "0.0.0",
    };

    fn metadata() -> TargetMetadata {
        TargetMetadata {
            specify_floor: None,
            inputs: Vec::new(),
            platforms: None,
        }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn guidance<P: Model>(_model: &P, _ctx: &Context<'_>) -> Result<String, Error> {
        Ok("fixture guidance".to_string())
    }

    async fn build<P: Model>(
        _model: &P, _ctx: &Context<'_>, slice: &str, inputs: &[Input], _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        Ok(echo(format!("build:{slice}:{}", inputs.len())))
    }

    async fn merge<P: Model>(
        _model: &P, _ctx: &Context<'_>, slice: &str, phase: MergePhase, _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        let phase = match phase {
            MergePhase::Preflight => "preflight",
            MergePhase::Postflight => "postflight",
        };
        Ok(echo(format!("merge:{slice}:{phase}")))
    }
}

/// A target whose `guidance` always fails with a typed internal error
/// naming the routed id.
#[derive(Clone, Copy, Debug)]
pub struct FailGuidance;

impl Target for FailGuidance {
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "fixture-fail-guidance",
        version: "0.0.0",
    };

    fn metadata() -> TargetMetadata {
        TargetMetadata {
            specify_floor: None,
            inputs: Vec::new(),
            platforms: None,
        }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn guidance<P: Model>(_model: &P, ctx: &Context<'_>) -> Result<String, Error> {
        Err(Error::Internal(format!("guidance failure for `{}`", ctx.adapter_id)))
    }

    async fn build<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, _inputs: &[Input], _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        Ok(Report::success())
    }

    async fn merge<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, _phase: MergePhase, _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        Ok(Report::success())
    }
}

/// A target declaring a `specify` compatibility floor, for
/// metadata-projection asserts.
#[derive(Clone, Copy, Debug)]
pub struct Floored;

impl Target for Floored {
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "floored",
        version: "0.0.0",
    };

    fn metadata() -> TargetMetadata {
        TargetMetadata {
            specify_floor: Some("9.9.9".to_string()),
            inputs: Vec::new(),
            platforms: None,
        }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn guidance<P: Model>(_model: &P, _ctx: &Context<'_>) -> Result<String, Error> {
        Ok("fixture guidance".to_string())
    }

    async fn build<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, _inputs: &[Input], _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        Ok(Report::success())
    }

    async fn merge<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, _phase: MergePhase, _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        Ok(Report::success())
    }
}

/// A source probe whose survey lead echoes the reference URL its
/// context carried (`none` when absent), for reference-routing
/// asserts.
#[derive(Clone, Copy, Debug)]
pub struct Reflect;

impl Source for Reflect {
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "reflect",
        version: "0.0.0",
    };

    fn metadata() -> SourceMetadata {
        SourceMetadata { specify_floor: None }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn survey<P: Model>(_model: &P, ctx: &Context<'_>) -> Result<Vec<Lead>, Error> {
        Ok(vec![Lead {
            lead: ctx.mcp_url.unwrap_or("none").to_string(),
            synopsis: format!("surveyed by {}", ctx.adapter_id),
            topics: Vec::new(),
        }])
    }

    async fn extract<P: Model>(
        _model: &P, _ctx: &Context<'_>, lead: &Lead,
    ) -> Result<Evidence, Error> {
        Err(Error::Internal(format!("no evidence for {}", lead.lead)))
    }
}

/// A target whose compile-time identity version is not SemVer, for
/// catalog-validation asserts.
#[derive(Clone, Copy, Debug)]
pub struct BadVersion;

impl Target for BadVersion {
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "bad-version",
        version: "dev",
    };

    fn metadata() -> TargetMetadata {
        TargetMetadata {
            specify_floor: None,
            inputs: Vec::new(),
            platforms: None,
        }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn guidance<P: Model>(_model: &P, _ctx: &Context<'_>) -> Result<String, Error> {
        Ok("fixture guidance".to_string())
    }

    async fn build<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, _inputs: &[Input], _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        Ok(Report::success())
    }

    async fn merge<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, _phase: MergePhase, _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        Ok(Report::success())
    }
}

/// A target reusing the `fixture` name at a different version, for
/// reference-shelf conflict asserts.
#[derive(Clone, Copy, Debug)]
pub struct ProbeV2;

impl Target for ProbeV2 {
    const IDENTITY: AdapterIdentity = AdapterIdentity {
        name: "fixture",
        version: "0.1.0",
    };

    fn metadata() -> TargetMetadata {
        TargetMetadata {
            specify_floor: None,
            inputs: Vec::new(),
            platforms: None,
        }
    }

    fn docs() -> &'static [Doc] {
        DOCS
    }

    async fn guidance<P: Model>(_model: &P, _ctx: &Context<'_>) -> Result<String, Error> {
        Ok("fixture guidance".to_string())
    }

    async fn build<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, _inputs: &[Input], _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        Ok(Report::success())
    }

    async fn merge<P: Model>(
        _model: &P, _ctx: &Context<'_>, _slice: &str, _phase: MergePhase, _tree: &WorkingTree,
    ) -> Result<Report, Error> {
        Ok(Report::success())
    }
}

/// A success report whose single output path records the invocation.
fn echo(path: String) -> Report {
    Report {
        status: Status::Success,
        findings: Vec::new(),
        outputs: vec![BuildOutput {
            platform: Platform::Core,
            path,
        }],
        ui_surface: None,
    }
}

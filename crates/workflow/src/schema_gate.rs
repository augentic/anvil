//! Domain-shaped JSON Schema validation hooks for source/target adapter split on-disk
//! artifacts.
//!
//! The raw JSON-Schema plumbing and embedded constants live in
//! [`schema`]; this module holds
//! the workflow-aware wrappers — they import [`crate::change::Plan`],
//! aggregate per-file findings into a single
//! [`error::Error::Validation`] payload, and pin the wire
//! `rule_id` strings the CLI surfaces.
//!
//! Schemas are embedded by [`schema::constants`] via
//! `include_str!`. The validators return [`error::Error::Validation`]
//! on a schema mismatch so the CLI exits with code 2
//! (`Exit::ValidationFailed` in the binary crate).
//!
//! One submodule per artifact family; this facade re-exports the
//! entry points so call sites keep the flat `schema_gate::validate_*`
//! paths.

mod design_system;
mod evidence;
mod judgment;
mod plan;
mod support;
mod target;
mod topology;

pub use design_system::validate_components_yaml;
pub use evidence::{
    EvidenceDoc, evidence_yaml_paths, validate_evidence, validate_evidence_dir, validate_leads,
};
pub use judgment::{validate_proposal_json, validate_synthesis_json};
pub use plan::{validate_plan, validate_plan_yaml};
pub use target::{validate_build_report_json, validate_build_request_json};
pub use topology::validate_topology_lock;

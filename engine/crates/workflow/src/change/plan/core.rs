//! On-disk representation of `plan.yaml` and the in-memory [`Plan`]
//! state machine that wraps it. [`Plan::transition`] is the only path
//! that mutates `Entry::status`.

pub mod amend;
pub mod archive;
pub mod authority_override;
pub mod create;
pub mod io;
pub mod model;
pub mod next;
pub mod propose;
pub mod remove;
pub mod status;
pub mod transitions;
pub mod validate;

pub use authority_override::{
    emit_seed_events as emit_authority_override_seed_events, entry_mut, mutate_authority_overrides,
    reject_orphan_overrides, unknown_slice_err,
};
pub use model::{
    Divergence, Entry, EntryPatch, Lifecycle, Patch, Plan, SliceAuthorityOverride,
    SliceSourceBinding, SourceBinding, Status, TargetRef, TargetRefParseError,
};
pub use next::{NextBody, NextReason, plan_next_body};
pub use propose::{
    LeadCatalog, LeadCatalogEntry, ProjectMissingPlatforms, ProjectRef, ProposalKind,
    ProposalRequest, ProposalResponse, ProposeOutcome, ResponseMember, ResponseSlice,
    apply_greenfield_seed, build_catalog, build_request, resolve_target, resolve_topology,
};
pub use status::{
    NextActionKind, StatusBody, StatusCounts, StopBody, StopReason, drained_line, plan_status_body,
};
#[cfg(test)]
pub use test_fixtures::{PLAN_EXAMPLE_YAML, change, change_with_deps, plan_with_changes};
pub use validate::{
    orphan_authority_override_keys, plan_finding, plan_finding_structured,
    reject_duplicate_source_keys,
};

#[cfg(test)]
mod test_fixtures {
    use std::collections::BTreeMap;

    use super::model::{Entry, Lifecycle, Plan, SliceAuthorityOverride, Status};

    /// Reduced-state reproduction of the plan execution
    /// §"The Plan" fixture. v1 has no per-entry `failed`, `blocked`, or
    /// `skipped` state — entries either move forward or stay where they
    /// are. The fixture has been mechanically rewritten to use the
    /// surviving three-state enum.
    pub const PLAN_EXAMPLE_YAML: &str = r"name: platform-v2
sources:
  monolith:
    adapter: typescript
    path: /path/to/legacy-codebase
  orders:
    adapter: typescript
    path: git@github.com:org/orders-service.git
  payments:
    adapter: typescript
    path: git@github.com:org/payments-service.git
  frontend:
    adapter: typescript
    path: git@github.com:org/web-app.git
slices:
  - name: user-registration
    project: platform
    sources: [monolith]
    status: done
  - name: email-verification
    project: platform
    sources: [monolith]
    depends-on: [user-registration]
    status: in-progress
  - name: registration-duplicate-email-crash
    project: platform
    description: >
      Duplicate email submission returns 500 instead of 409.
      Discovered during email-verification extraction.
    status: pending
  - name: notification-preferences
    project: platform
    depends-on: [user-registration]
    description: >
      Greenfield — user-facing notification channel and frequency settings.
    status: pending
  - name: extract-shared-validation
    project: platform
    description: >
      Pull duplicated input validation into a shared validation crate
      before building checkout-flow.
    depends-on: [email-verification]
    status: pending
  - name: product-catalog
    project: platform
    sources: [monolith]
    depends-on: [extract-shared-validation]
    status: pending
  - name: shopping-cart
    project: platform
    sources: [orders]
    depends-on: [product-catalog, user-registration]
    status: pending
  - name: checkout-api
    project: platform
    sources: [payments]
    depends-on: [shopping-cart]
    status: pending
  - name: checkout-ui
    project: platform
    sources: [frontend]
    depends-on: [checkout-api]
    status: pending
";

    pub fn plan_with_changes(changes: Vec<Entry>) -> Plan {
        Plan {
            name: "test".into(),
            lifecycle: Lifecycle::Pending,
            sources: BTreeMap::new(),
            entries: changes,
        }
    }

    pub fn change(name: &str, status: Status) -> Entry {
        Entry {
            name: name.into(),
            project: Some("default".into()),
            status,
            depends_on: vec![],
            sources: vec![],
            context: vec![],
            description: None,
            divergence: None,
            disagreements: Vec::new(),
            authority_override: SliceAuthorityOverride::default(),
        }
    }

    pub fn change_with_deps(name: &str, status: Status, deps: &[&str]) -> Entry {
        let mut e = change(name, status);
        e.depends_on = deps.iter().map(|s| (*s).into()).collect();
        e
    }
}

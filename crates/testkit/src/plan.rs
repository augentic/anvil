//! Plan fixtures: the workflow-contract example plan and minimal
//! in-memory plan builders.

/// Reduced-state reproduction of the plan execution §"The Plan"
/// fixture. There is no per-entry `failed`, `blocked`, or `skipped`
/// state — entries either move forward or stay where they are.
pub const PLAN_EXAMPLE_YAML: &str = r"name: platform-v2
sources:
  monolith:
    adapter: demo-source
    path: /path/to/legacy-codebase
  orders:
    adapter: demo-source
    path: git@github.com:org/orders-service.git
  payments:
    adapter: demo-source
    path: git@github.com:org/payments-service.git
  frontend:
    adapter: demo-source
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

/// A minimal in-memory plan named `test` wrapping `changes`.
#[must_use]
pub fn plan_with_changes(changes: Vec<project::plan::Entry>) -> project::plan::Plan {
    project::plan::Plan {
        name: "test".into(),
        lifecycle: project::plan::Lifecycle::Pending,
        sources: std::collections::BTreeMap::new(),
        entries: changes,
    }
}

/// A minimal plan entry bound to project `default`.
#[must_use]
pub fn change(name: &str, status: project::plan::Status) -> project::plan::Entry {
    project::plan::Entry {
        name: name.into(),
        project: Some("default".into()),
        status,
        depends_on: vec![],
        sources: vec![],
        context: vec![],
        description: None,
        divergence: None,
        disagreements: Vec::new(),
        authority_override: project::plan::AuthorityOverride::default(),
    }
}

/// [`change()`] plus a `depends-on` list.
#[must_use]
pub fn change_with_deps(
    name: &str, status: project::plan::Status, deps: &[&str],
) -> project::plan::Entry {
    let mut e = change(name, status);
    e.depends_on = deps.iter().map(|s| (*s).into()).collect();
    e
}

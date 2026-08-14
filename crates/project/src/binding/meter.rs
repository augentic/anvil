//! Wave-bind budget counters charged against [`super::Policy`].

use std::time::Instant;

use error::Error;

use super::Policy;

/// Running consumption of one wave-bind against a [`Policy`].
#[derive(Debug)]
#[expect(
    missing_copy_implementations,
    reason = "copying would fork budget counters independently of the wave bind"
)]
pub struct Meter {
    started: Instant,
    bindings: usize,
    api: usize,
    bytes: u64,
    trees: usize,
}

impl Meter {
    /// Start a wave-bind clock.
    #[must_use]
    pub fn new() -> Self {
        Self {
            started: Instant::now(),
            bindings: 0,
            api: 0,
            bytes: 0,
            trees: 0,
        }
    }

    /// Charge one locator row.
    ///
    /// # Errors
    ///
    /// `binding-budget-exhausted` when bindings or wall-clock exceed the policy.
    pub fn binding(&mut self, policy: &Policy) -> Result<(), Error> {
        self.bindings += 1;
        if self.bindings > policy.bindings {
            return Err(exhausted("bindings", policy.bindings));
        }
        self.time(policy)
    }

    /// Charge one repository-host or HTTPS request.
    ///
    /// # Errors
    ///
    /// `binding-budget-exhausted` when API requests or wall-clock exceed the policy.
    pub fn api(&mut self, policy: &Policy) -> Result<(), Error> {
        self.api += 1;
        if self.api > policy.api_requests {
            return Err(exhausted("api-requests", policy.api_requests));
        }
        self.time(policy)
    }

    /// Charge `n` inspected bytes.
    ///
    /// # Errors
    ///
    /// `binding-budget-exhausted` when inspected-bytes or wall-clock exceed the policy.
    pub fn bytes(&mut self, n: u64, policy: &Policy) -> Result<(), Error> {
        self.bytes = self.bytes.saturating_add(n);
        if self.bytes > policy.inspected_bytes {
            return Err(exhausted("inspected-bytes", policy.inspected_bytes));
        }
        self.time(policy)
    }

    /// Charge one imported tree (including a one-file tree).
    ///
    /// # Errors
    ///
    /// `binding-budget-exhausted` when imported-trees or wall-clock exceed the policy.
    pub fn tree(&mut self, policy: &Policy) -> Result<(), Error> {
        self.trees += 1;
        if self.trees > policy.imported_trees {
            return Err(exhausted("imported-trees", policy.imported_trees));
        }
        self.time(policy)
    }

    /// Fail when the wall-clock budget is gone.
    ///
    /// # Errors
    ///
    /// `binding-budget-exhausted` when elapsed time exceeds `policy.time_ms`.
    pub fn time(&self, policy: &Policy) -> Result<(), Error> {
        let elapsed = u64::try_from(self.started.elapsed().as_millis()).unwrap_or(u64::MAX);
        if elapsed > policy.time_ms {
            return Err(exhausted("time", policy.time_ms));
        }
        Ok(())
    }
}

impl Default for Meter {
    fn default() -> Self {
        Self::new()
    }
}

fn exhausted(budget: &str, cap: impl std::fmt::Display) -> Error {
    Error::Diag {
        code: "binding-budget-exhausted",
        detail: format!("delivery-binding {budget} budget ({cap}) exhausted"),
    }
}

//! Consolidated integration binary for `dispatch`.
//!
//! One binary per crate: each area file is pulled in as a submodule so
//! the harness links once. `guest` pins the shared-grammar parse and
//! the route table's three-way split; `verbs` drives the pure
//! project-scoped verb handlers end to end through that same route
//! table (filesystem effects + exit codes). See
//! [docs/standards/testing.md](../../../docs/standards/testing.md).

mod guest;
mod verbs;

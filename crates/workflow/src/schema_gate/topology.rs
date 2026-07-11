//! Workspace topology gate: the `topology.lock` cache shape.

use error::Result;
use schema::{TOPOLOGY_LOCK_JSON_SCHEMA, validate_serialisable};

/// Validate a [`crate::registry::TopologyLock`] against the embedded
/// `schemas/topology-lock.schema.json`.
///
/// Returns `Ok(())` on a clean validation; otherwise a payload-free
/// [`error::Error::Validation`] keyed on `"topology-lock-schema"`. Used
/// by the `topology.lock` reader/writer so a corrupt cache fails
/// closed.
///
/// # Errors
///
/// Returns [`error::Error::Validation`] when the lock fails the schema;
/// falls back to [`error::Error::Diag`] when the embedded schema is
/// unparseable or the lock is not JSON-serialisable (both unreachable
/// in production).
pub fn validate_topology_lock(lock: &crate::registry::TopologyLock) -> Result<()> {
    validate_serialisable(
        lock,
        TOPOLOGY_LOCK_JSON_SCHEMA,
        "topology-lock-schema",
        ".specify/topology.lock conforms to schemas/topology-lock.schema.json",
        "topology-lock-schema-serialise",
        "topology.lock",
    )
}

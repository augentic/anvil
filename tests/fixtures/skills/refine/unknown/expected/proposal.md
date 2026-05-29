# Audit trail retention

## Why

Settle the retention policy for the identity service's audit trail. The candidate exists in the plan (operator-flagged at `/spec:plan`) but no contributing source surfaced a concrete claim about the retention window or eviction strategy; the operator is expected to fill the gap before `/spec:build`.

## Units

- audit-trail-retention — audit-trail retention window and eviction trigger definition

## Non-goals

- Restructuring the audit-trail storage layer.
- Backfilling retention against existing audit rows already past the window.

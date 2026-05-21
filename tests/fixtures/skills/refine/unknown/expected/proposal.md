# Audit trail retention

## Motivation

Settle the retention policy for the identity service's audit trail. The candidate exists in the plan (operator-flagged at `/spec:plan`) but no contributing source surfaced a concrete claim about the retention window or eviction strategy; the operator is expected to fill the gap before `/spec:build`.

## Scope

- Define the audit-trail retention window and the eviction trigger that enforces it.

## Non-goals

- Restructuring the audit-trail storage layer.
- Backfilling retention against existing audit rows already past the window.

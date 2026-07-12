# Audit trail retention — design

## Domain model

- `RetentionWindow` newtype wrapping the retention duration once the operator settles REQ-001.

## Provider trait dependencies

- The retention-enforcing worker depends on `TableStore` (scan + delete on the audit-trail table) and `Config` (read of the retention window).

## APIs and integrations

- No external HTTP surface. Retention runs as a scheduled job; the schedule cadence is operator-confirmed at build time once REQ-001 is reconciled.

## Configuration

- `AUDIT_TRAIL_RETENTION_DURATION` — `Config::get` key for the retention window. The default value is unresolved while REQ-001 is `[unknown]`.

## Operation delegation

- Not applicable; this slice is configuration + a scheduled worker. Concrete operation shape follows once REQ-001 is settled.

## Error mapping

- `Error::ServerError` for unexpected `TableStore` failures during retention sweeps.

## Validation placement

- Config-read time: the retention duration must parse as a positive duration string (e.g. `30d`, `90d`).

## Observability

- `tracing::info!(monotonic_counter.audit_trail_evictions_total = <n>)` per sweep, with `n` the number of rows deleted.

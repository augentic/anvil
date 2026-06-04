# Tasks

- [ ] Reconcile the `[unknown]` on REQ-001 — confirm the audit-trail retention window with the operator, then hand-edit `spec.md` to flip to `Status: agreed` and populate the body, or amend the plan with a source that supplies the value and re-run `/spec:refine`.
- [ ] Once REQ-001 is reconciled, set `AUDIT_TRAIL_RETENTION_DURATION` in the Omnia config.
- [ ] Author the retention worker per design.md.
- [ ] Add tests covering the eviction sweep (no-op when nothing is past the window; correct delete count when rows are past).
- [ ] Run code review.

# Tasks

- [ ] Reconcile the `[unknown]` on REQ-001 — confirm the audit-trail retention window with the operator, then amend the plan with a source that supplies the value (`specify plan amend audit-trail-retention --add-source <key>=<path>`) and re-run `/spec:refine`. The `[unknown]` tag and `Status:` are kernel-rendered — do not hand-edit them to `agreed`.
- [ ] Once REQ-001 is reconciled, set `AUDIT_TRAIL_RETENTION_DURATION` in the Omnia config.
- [ ] Author the retention worker per design.md.
- [ ] Add tests covering the eviction sweep (no-op when nothing is past the window; correct delete count when rows are past).
- [ ] Run code review.

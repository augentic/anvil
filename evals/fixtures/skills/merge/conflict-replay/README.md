# `merge/conflict-replay/`

Pins the merge-conflict stop hint contract from [`plugins/spec/skills/merge/SKILL.md`](../../../../../plugins/spec/skills/merge/SKILL.md) §Stop hint contract. Stress-tests the merge-baseline-conflict branch of workflow §Execution model trigger table.

## Scenario

`/spec:merge identity-user-registration` runs against a slice already at `status: built`. The omnia pre-merge gate passes (cargo, clippy, tests, wasm32 build all green). When the body invokes `specify slice merge run identity-user-registration --format json`, the CLI's deterministic delta-merge step detects that a sibling slice merged between this slice's `defined-at` and now: `.specify/specs/omnia/spec.md` was modified at a baseline timestamp newer than the slice's `defined-at`, and one requirement block (`REQ-007 Reset link expiry`) overlaps with the slice's delta.

The skill body MUST:

1. Not retry `specify slice merge` automatically.
2. Not call `specify slice transition` — the slice stays at `built`.
3. Not write to `plan.yaml` — the plan entry stays `in-progress`.
4. Emit the structured stop hint with `failure-kind: baseline-conflict` and the conflicting baseline paths.
5. Release the plan lock on exit (the `flock`-bound fd 9 closes when the body returns).

The operator's recovery path is to inspect the conflicting baseline, hand-edit the slice's delta to align, and re-invoke `/spec:merge identity-user-registration` (or let `/spec:execute` re-enter the loop).

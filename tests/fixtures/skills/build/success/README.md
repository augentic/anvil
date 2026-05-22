# `build/success/`

Pins the happy-path output of `/spec:build` against an `omnia` slice. Stress-tests the success-side of RFC-25 §Acceptance scenario `#9` (the failure-side lives at [`../failure-replay/`](../failure-replay/)).

## Scenario

`/spec:build password-hash-rotate` runs against a slice already at `status: refined`. The target's build brief completes the verify-repair loop and code-review pass without a non-zero exit. Tasks 1–N flip to checked in `tasks.md` as the brief progresses.

The skill body MUST:

1. Resolve the slice via `specify plan next` (or validate the supplied `[slice-name]` arg matches it).
2. Acquire the plan lock when invoked standalone (`SPECIFY_PLAN_LOCK_HELD` unset).
3. Read and execute `adapters/targets/omnia/briefs/build.md` linearly.
4. Run `specify slice transition password-hash-rotate built --format json` exactly once on success.
5. Return control to the caller without writing `plan.yaml` (the per-entry `done` is `/spec:merge`'s job).

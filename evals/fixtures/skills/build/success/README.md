# `build/success/`

Pins the happy-path output of `/spec:build` against an `omnia` slice. Stress-tests the success-side of workflow §Eval scenario `#9` (the failure-side lives at [`../failure-replay/`](../failure-replay/)).

## Scenario

`/spec:build password-hash-rotate` runs against a slice already at `status: refined`. The target's build brief completes the verify-repair loop and code-review pass without a non-zero exit. Tasks 1–N flip to checked in `tasks.md` as the brief progresses.

The skill body MUST:

1. Acquire the plan lock when invoked standalone (`SPECIFY_PLAN_LOCK_HELD` unset) — before any plan verb; `specify plan next` is CLI-gated and refuses an unlocked driver with `plan-lock-not-held`.
2. Resolve the slice via `specify plan next` (or validate the supplied `[slice-name]` arg matches it).
3. Run `specify slice build password-hash-rotate --phase prepare --format json`, then read and execute the handoff's `adapters/targets/omnia/briefs/build.md` linearly.
4. Run `specify slice build password-hash-rotate --phase finalize --format json` exactly once on a clean report — finalize owns the `Refined → Built` gate; the body never calls `specify slice transition ... built`.
5. Return control to the caller without writing `plan.yaml` (the per-entry `done` is `/spec:merge`'s job).

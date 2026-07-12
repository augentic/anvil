# `/spec:merge` worked-example fixtures

Pinned scenarios for the `/spec:merge` skill body at [`plugins/spec/skills/merge/SKILL.md`](../../../../../plugins/spec/skills/merge/SKILL.md). Each fixture documents the inputs the skill is invoked with (active plan entry + slice `metadata.yaml`) and the visible output the body must emit.

## Fixture matrix

| Fixture              | Trigger                                                         | Eval scenario | Expected output                                                                  |
| -------------------- | --------------------------------------------------------------- | ------------------- | -------------------------------------------------------------------------------- |
| `conflict-replay/`   | `specify slice merge` returns non-zero on a baseline overlap.   | #11 (paired with build) | Stop hint with `failure-kind: baseline-conflict` + conflict paths; slice stays `built`; plan entry stays `in-progress`. |
| `success/`           | Pre-merge gate passes; `specify slice merge` exits zero.        | #9 happy-path       | Slice transitions to `merged`, archive moved, plan entry stamped `done`.         |

`/spec:merge` invoked under `SPECIFY_PLAN_LOCK_HELD=1` follows the same env-var-detection contract as `/spec:build`; see [`../build/breakout-from-execute/`](../build/breakout-from-execute/) for the shared contract.

## Layout per fixture

```text
<fixture-name>/
  README.md               # what the fixture exercises
  input/
    plan.yaml             # the plan state /spec:merge sees on entry
    slice-metadata.yaml   # the slice's .specify/slices/<name>/metadata.yaml on entry
  expected-stop-hint.md   # for failure cases: the stop hint the body emits
  expected-trace.md       # for success cases: the body's visible behaviour
```

The fixtures are documentation pins for the skill body. They are not yet executable end-to-end — the runner that consumes them lands when `/spec:execute` and the per-target merge briefs are wired into a CLI-side eval harness.

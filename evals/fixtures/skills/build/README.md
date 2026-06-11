# `/spec:build` worked-example fixtures

Pinned scenarios for the `/spec:build` skill body at [`plugins/spec/skills/build/SKILL.md`](../../../../plugins/spec/skills/build/SKILL.md). Each fixture documents the inputs the skill is invoked with (active plan entry + slice `metadata.yaml` + env vars) and the visible output the body must emit.

## Fixture matrix

| Fixture                     | Trigger                                             | Eval scenario | Expected output                                            |
| --------------------------- | --------------------------------------------------- | ------------------- | ---------------------------------------------------------- |
| `failure-replay/`           | Target build brief exits non-zero on `cargo test`.  | #9 (build park)     | Stop hint with `failing-task` + `log-path`; slice stays `refined`; plan entry stays `in-progress`. |
| `success/`                  | Target build brief completes; slice goes to `built`.| #9 happy-path       | `specify slice transition <slice> built`; control returns to caller. |
| `breakout-from-execute/`    | `/spec:build` invoked from inside `/spec:execute` with `SPECIFY_PLAN_LOCK_HELD=1`. | #11 | Skip plan-lock acquire; otherwise identical to standalone path. |

## Layout per fixture

```text
<fixture-name>/
  README.md               # what the fixture exercises
  input/
    plan.yaml             # the plan state /spec:build sees on entry
    slice-metadata.yaml   # the slice's .specify/slices/<name>/metadata.yaml on entry
    env.txt               # (optional) env vars set by the parent (SPECIFY_PLAN_LOCK_HELD, ...)
  expected-stop-hint.md   # for failure cases: the stop hint the body emits
  expected-trace.md       # for success / breakout cases: the body's visible behaviour
```

The fixtures are documentation pins for the skill body. They are not yet executable end-to-end — the runner that consumes them lands when `/spec:execute` and the per-target build briefs are wired into a CLI-side eval harness.

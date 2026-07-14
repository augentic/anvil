# Prompt evaluation

A live-model example of the Specify engine workflow over the fixture adversarial lead set. Graded by deterministic validators only — never a second model judging the first.

Layout: `engine.rs` is the workflow driver over the live cursor-agent backend; `native.rs` is the example-local `Native` adapter that runs `omnia_cursor::Client` behind the guest-side `Model` trait (the guest→wire mapping, the request/answer gates, and the workspace lend).

## Workflow

The driver mirrors the operator rhythm:

```text
plan        specify plan author → Gate 1 approved
execute     per slice: plan next → refine → build → merge  (until drained)
finalize    specify plan archive
```

`execute` is hand-driven with the breakout verbs so each phase is visible. Production `specify plan execute` drains the same refine → build → merge loop automatically.

## Run

Install and authenticate `cursor-agent`, then run from the repository root:

```bash
cargo make prompt-eval
```

## Grading contract

Hard assertions only:

| Stage | Check | Pass condition |
| ----- | ----- | -------------- |
| plan | Cross-source overlap | `login-flow` from `docs` and `code` merge into one slice |
| execute | Lifecycle | Every plan entry is `done` |
| execute | Provenance | Every evidenced requirement carries sources; ids are present |
| execute | Authority disagreement | Session-timeout surfaces as `[divergence]` or `[conflict]` |
| execute | Evidence gap | Password-reset is marked `[unknown]`, not invented |
| execute | Build output | Every slice leaves a non-empty fixture build artifact |

Per-leg request / repair counts are **reported, not asserted**. A leg drifting from zero repairs toward the budget is the early signal that a prompt or answer-schema change degraded the model's first answer.

The temporary project path is printed at startup. Successful runs remove it; failed runs retain it for inspection.

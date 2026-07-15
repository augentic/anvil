# Prompt evaluation

A live-model harness over the Specify engine workflow and the fixture adversarial lead set. Graded by deterministic validators only — never a second model judging the first.

Layout: `src/main.rs` is the workflow driver over the live cursor-agent backend; `src/native.rs` is the harness-local `Native` adapter that runs `omnia_cursor::Client` behind the guest-side `Model` trait (the guest→wire mapping, the request/answer gates, and the workspace lend). Fixture plumbing — the adapter core, the model-generic provider, the adversarial bindings — comes from `crates/testkit`; only the model is live.

## Workflow

The driver mirrors the operator rhythm:

```text
init        specify init fixture
plan        specify plan author → Gate 1 approved
execute     specify plan execute  (refine → build → merge per slice, until drained)
finalize    specify plan archive
```

Every step runs the production operation — `execute` is the real drained loop, not a hand-driven breakout sequence. Completed phases are echoed as the loop runs.

## Run

Install and authenticate `cursor-agent`, then run from the repository root:

```bash
cargo make eval
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

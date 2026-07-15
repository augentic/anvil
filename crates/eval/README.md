# Prompt evaluation

A live-model harness for use in testing specify core prompts used when model judgement is required. For example, iin lead reconciliation or slice synthesis. Outputs are graded by deterministic validators — not a model.

## Quick start

Login to the Cursor agent:

```bash
agent login
```

or set `CURSOR_API_KEY` in `.env`.

```bash
make eval
```

This will run the entire workflow in `sandbox/eval/`. A passing run will remove the project, while a failing run will retain it.

## Manual workflow

Run one operation at a time to inspect its artifacts:

```bash
make eval init
make eval plan
make eval execute
make eval finalize
```

Manual operations share `sandbox/eval/` and leave it in place. `init` replaces any previous manual project; each later operation requires the preceding state. Remove the project when finished:

```bash
make eval clean
```

## Workflow

The driver mirrors the operator rhythm:

```text
init        specify init fixture
plan        specify plan author → Gate 1 approved
execute     specify plan execute  (refine → build → merge per slice, until drained)
finalize    specify plan archive
```

Every step runs the production operation — `execute` is the real drained loop, not a hand-driven breakout sequence. Completed phases are echoed as the loop runs.

## Grading contract

Hard assertions only:


| Stage   | Check                  | Pass condition                                               |
| ------- | ---------------------- | ------------------------------------------------------------ |
| plan    | Cross-source overlap   | `login-flow` from `docs` and `code` merge into one slice     |
| execute | Lifecycle              | Every plan entry is `done`                                   |
| execute | Provenance             | Every evidenced requirement carries sources; ids are present |
| execute | Authority disagreement | Session-timeout surfaces as `[divergence]` or `[conflict]`   |
| execute | Evidence gap           | Password-reset is marked `[unknown]`, not invented           |
| execute | Build output           | Every slice leaves a non-empty fixture build artifact        |


Per-leg request / repair counts are **reported, not asserted**. After grading, the trial prints one line per judgment leg (keyed by answer-schema name) with its request count and derived repairs — requests beyond one per leg invocation (one propose per trial, one synthesis per plan entry), e.g. `leg synthesis: 4 request(s) over 3 slice(s), 1 repair(s)`. A leg drifting from zero repairs toward the budget is the early signal that a prompt or answer-schema change degraded the model's first answer.

In manual mode, repair counts cover only model requests made by that operation.
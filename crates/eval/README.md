# Prompt evaluation

A live-model harness over the Specify engine workflow and the fixture adversarial lead set. Graded by deterministic validators only — never a second model judging the first.

## Quick start

Login to the Cursor agent:

```bash
agent login
```

or set `CURSOR_API_KEY` in `examples/.env`.

```bash
cargo make eval
```

The default command runs the entire workflow in a temporary project. A passing run removes the project; a failing run retains it at the path printed on startup.

## Manual workflow

Run one operation at a time to inspect its artifacts:

```bash
cargo make eval init
cargo make eval plan
cargo make eval execute
cargo make eval finalize
```

Manual operations share `target/eval/` and leave it in place. `init` replaces any previous manual project; each later operation requires the preceding state. Remove the project when finished:

```bash
cargo make eval clean
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

Per-leg request / repair counts are **reported, not asserted**: after grading, the trial prints one line per judgment leg (keyed by answer-schema name) with its request count and derived repairs — requests beyond one per leg invocation (one propose per trial, one synthesis per plan entry), e.g. `leg synthesis: 4 request(s) over 3 slice(s), 1 repair(s)`. A leg drifting from zero repairs toward the budget is the early signal that a prompt or answer-schema change degraded the model's first answer.

The project path is printed at startup. In manual mode, repair counts cover only model requests made by that operation.
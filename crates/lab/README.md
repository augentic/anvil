# Prompt evaluation

A live-model harness for use in testing specify core prompts used in core model judgement steps. Outputs are graded by deterministic validators — not a model.

## Quick start

Login to the Cursor agent:

```bash
[cursor-]agent login
```

or set `CURSOR_API_KEY` in `.env` at the repository root.

```bash
make eval
```

This runs the entire workflow in `sandbox/`. A passing run will remove the project, while a failing run will retain it for in-place review, or to re-run individual operations (using the manual workflow below).

`SPECIFY_EVAL_MODEL=<model-id>` overrides the model for a run: unset means the cursor backend's default.

### Manual workflow

Run one operation at a time to inspect its artifacts:

```bash
make eval init
make eval plan
make eval execute
make eval finalize
```

While `make eval init` will reinitialize a project, a project can also be removed using:

```bash
make eval clean
```

## Model judgment

Specify core has two steps that require a model's judgement.


| Schema      | Step       | Crate    | Purpose                                                                                                                                     |
| ----------- | ---------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `proposal`  | Propose    | `change` | Reconcile *surveyed* leads across sources into plan slices and author the Gate 1 prose.                                                     |
| `synthesis` | Synthesize | `slice`  | Reconcile *extracted* evidence with baseline context and target guidance, to produce build artifacts such as `proposal.md`, `spec.md`, etc. |


The execution and repair loop lives in the `project` crate and are considered infrastructure, not judgment. Source `survey` / `extract` and target `guidance` / `build` / `merge` are adapter operations.

The `plan` evaluation isolates the proposal leg. The `execute` evaluation runs synthesis as part of the complete `refine → build → merge` loop. The synthesis artifacts are fields of one model response, so the harness does not invoke `spec.md` or `design.md` generation as independent judgment legs.

## Workflow

The driver mirrors the operator rhythm:

```text
init        specify init fixture
plan        specify plan author → Gate 1 approved
execute     specify plan execute  (refine → build → merge per slice, until drained)
finalize    specify plan archive
```

Every step runs the production operation — `execute` is the real drained loop, not a hand-driven breakout sequence. Completed phases are echoed as the loop runs.

## Grading

Hard assertions only:


| Stage   | Check      | Pass condition                                               |
| ------- | ---------- | ------------------------------------------------------------ |
| plan    | Entries    | `plan author` produces at least one entry                    |
| execute | Lifecycle  | Every plan entry is `done`                                   |
| execute | Provenance | Every evidenced requirement carries sources; ids are present |


Per-leg request / repair counts are **reported, not asserted**. After grading, the trial prints one line per judgment leg (keyed by answer-schema name) with its request count and derived repairs — requests beyond one per leg invocation (one propose per trial, one synthesis per plan entry), e.g. `leg synthesis: 4 request(s) over 3 slice(s), 1 repair(s)`. A leg drifting from zero repairs toward the budget is the early signal that a prompt or answer-schema change degraded the model's first answer.

In manual mode, repair counts cover only model requests made by that operation.
# Prompt evaluation

The lab-only `probe` library: trial/scenario runners, deterministic grading,
and telemetry over the native host. The composition example that drives it
is [`examples/eval/`](../../examples/eval/README.md) (`cargo make eval` /
`cargo make specify`). Outputs are graded by deterministic validators — not
a model.

## Quick start

Login to the Cursor agent:

```bash
[cursor-]agent login
```

or set `CURSOR_API_KEY` in `.env` at the repository root.

```bash
cargo make eval
```

This runs the entire workflow in `sandbox/`. A passing run will remove the project, while a failing run will retain it for in-place review, or to re-run individual operations (using the manual workflow below).

Driver-side knobs (read by `probe::client`):

| Env                       | Effect                                                                                                          |
| ------------------------- | --------------------------------------------------------------------------------------------------------------- |
| `EVAL_MODEL=<model-id>`   | Override the model for a run; unset means the cursor backend's default.                                         |
| `EVAL_TIMEOUT_SECS=<u64>` | Per-spawn `cursor-agent` wall-clock bound (seconds). Unset → backend default 120. `cargo make eval` sets `300`. |

The auth trial's hard synthesis case is **authority divergence** — the `session-timeout` lead (scripted goldens name the slice `session-policy`), not evidence volume. Docs claim a 30-minute idle timeout; code claims 15; documentation authority should win. Grading still only asserts lifecycle + provenance; it does not require a `[divergence]` tag.

`judgment-model-failed` on refine usually means the cursor layer timed out or returned unparseable JSON — the engine `MAX_REPAIRS` loop never sees that failure.

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
init        specify init mock
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

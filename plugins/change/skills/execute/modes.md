# Modes

Each mode below describes only the *delta* from the [per-slice algorithm](per-slice-algorithm.md). The 13 steps in that file are the common core; the mode-specific rules are the outer loop, the writing-vs-reporting substitution table, and the interrupt handling.

## Supervised (default)

Run the per-slice algorithm once, exactly as written, then exit. A supervised run reaches a terminal plan status (`done` / `failed` / `blocked`) for exactly one slice and stops; the driver never iterates to a second slice. `--loop` (below) is the mode that layers an outer iteration on top of the same per-slice steps.

## `--dry-run`

Run the per-slice algorithm with every **write** substituted for a **report** (see the §Guardrails section in `SKILL.md` for the explicit MUST-NOTs). Concretely:

| Write | Dry-run substitute |
|---|---|
| `specify change plan transition <name> in-progress` (step 5) | Not invoked. The preview shows the plan in its current state. |
| `specify change plan transition <name> {done,failed,blocked}` (steps 10–12) | Not invoked. Diagnostics use the "Would transition" wording (see [self-heal.md](self-heal.md) §Dry-run variant). |
| `specify slice journal append … recovery …` (self-heal step 4) | Not invoked. |
| `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop` (steps 6–8, 11b, 12b) | Not invoked. `--dry-run` is read-only end to end — the self-heal scan is report-only too. |

Step 4's `specify change plan next` / `specify change plan status` calls still run — they are read-only. The rendered output follows the §`--dry-run` output format in [output-format.md](output-format.md); every line carries the `[dry-run] ` banner so the operator cannot mistake a preview for a real run.

If self-heal would halt (ambiguity case), dry-run emits the same halt diagnostic, releases the lock, and exits non-zero WITHOUT reaching step 4 — the one non-zero exit on the happy startup path.

## `--loop`

Wrap the per-slice algorithm (steps 3–12) in an outer iteration:

```text
4a. Iteration body:
      loop:
        - run steps 3–12 against whatever `specify change plan next`
          returns; on return (terminal plan status reached), DO NOT
          release the lock.
        - loop back to step 4 of the per-slice algorithm
          (`specify change plan next --format json`).
        - break on `reason ∈ {"all-done", "stuck"}` (classifications
          of the same name) or defence-in-depth `reason ==
          "in-progress"` (classification `halted`).
4b. After the loop breaks, emit the terminal summary
    ([output-format.md](output-format.md) §Terminal summary) and go
    to step 13 (lock release) ONCE.
```

Mode invariants:

- **Lock is held for the entire run.** `specify change plan lock acquire` runs once at step 2 of the per-slice algorithm; `specify change plan lock release` runs once at step 13. The outer iteration neither acquires nor releases the lock.
- **Self-heal runs once.** The step-3 pass happens before the first iteration. Subsequent iterations do not re-run self-heal.
- **`failure` does NOT stop the loop.** An individual slice that returns `outcome: failure` is transitioned to `failed` inside steps 11a–c; the driver then continues to the next `specify change plan next` call. `specify change plan next` naturally skips `failed` entries, so the loop advances without extra branching.
- **`deferred` does NOT stop the loop.** Same shape with `blocked` instead of `failed`.
- **Loop stops when `specify change plan next` reports no eligible slice or a non-phase guard halts.** Terminal classifications are `all-done` (every entry in `{done, skipped}`), `stuck` (pending / blocked / failed entries remain but no pending entry has its `depends-on` satisfied), or `halted` (self-heal ambiguity, branch-preparation refusal, baseline residue after merge, or residue commit failure).
- **Phase failures / deferrals do not halt.** Mid-loop `failure` and `deferred` outcomes reach terminal plan statuses (`failed` / `blocked`) and the loop continues. `halted` is reserved for conditions where the driver cannot safely write a terminal status.
- **No phase-level parallelism.** At most one slice is `in-progress` at a time; the loop does not fan out concurrent phase invocations.

### SIGINT / SIGTERM handling

The skill runs inside an agent session; the agent process (not this skill directly) traps SIGINT / SIGTERM. The contract the skill must honour when the agent surfaces an interrupt is:

```text
1. Finish the current PHASE. Do NOT tear a /spec:define, /spec:build,
   or /spec:merge mid-invocation — doing so can leave slice
   artifacts in a half-written state that self-heal then has to
   reconcile.

2. Skip subsequent phases of the CURRENT slice. If build has not
   yet started when the interrupt arrives, do NOT start it; if
   build has just finished, do NOT invoke /spec:merge. The already-
   completed phase has stamped its outcome on disk, so self-heal on
   the next run will either resume (success on define/build) or
   resolve terminally (success on merge, failure, deferred).

3. Leave the active slice entry as in-progress. Do NOT run
   `specify change plan transition` on interrupt — the write path is
   reserved for normal outcomes. Self-heal on the next run will
   reclaim the entry based on .metadata.yaml.outcome.

4. Release the driver lock:
     specify change plan lock release --pid <agent-session-pid>
   Run this before exit regardless of which phase was mid-flight.

5. Emit the terminal summary with Completion: driver-interrupted and
   Next action pointing the operator at `/change:execute --loop` to
   resume. The summary's Progress line reflects the state as of the
   interrupt — the active entry still shows in-progress.

6. Exit non-zero (typically 130 for SIGINT, 143 for SIGTERM).
```

The skill cannot trap signals directly (agent-side shells handle signal delivery), but the above is the contract the skill's *logic* must satisfy so that the observable on-disk state after an interrupt is always recoverable by self-heal on the next run.

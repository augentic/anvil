# Driver mutual exclusion — CLI contract

The native `plan lock` command-wrapper and its `require_held` probes retired with the old stack at the Omnia-migration cutover. Driver mutual exclusion is now guest-owned.

## The guest marker

The `plan execute` orchestrator takes the `.specify/guest.lock` marker for the lifetime of the loop. A second driver session that finds the marker held exits immediately with the structured error `guest-marker-held` (exit 2). The marker is released when the loop exits — cleanly, on error, or on signal.

## Breakouts

Standalone phase invocations (`slice refine`, `slice build`, `slice merge run`) do not take the marker: the lifecycle gates (only `refined` builds, only `built` merges, per-entry `done` written only by merge) are the correctness fence, and the loop's re-entry semantics resume a parked slice at its next phase.

## Stale marker

If the holder process died without releasing (`kill -9`, OOM, host crash), the operator confirms the holder is dead and removes `.specify/guest.lock` by hand. There is no watchdog and no liveness probe.

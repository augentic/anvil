# driver-busy — second `/change:execute loop` refused while the first runs

A first `/change:execute loop` invocation has already acquired `.specify/plan.lock` and is iterating through the plan. A second invocation — from another agent session, a human's shell, or a stale retry — runs concurrently. The lock abstraction (RFC-2 §"Driver Concurrency") refuses the second acquire with `Error::DriverBusy { pid }`. The second `/change:execute` reports the conflict and exits 1 without touching the plan.

This fixture ships no `plan.yaml.before` / `plan.yaml.after` because the refused run never reaches a state where plan contents matter. It pins the diagnostic shape an operator sees when they collide with a running driver.

## Driver timeline

```text
# (First /change:execute loop invocation is running in another
# agent session — PID 48217 — and has held .specify/plan.lock for
# several minutes. It is currently partway through /spec:build on
# some change; irrelevant here.)

# Second invocation:
$ /change:execute loop

# step 1 of the --loop algorithm: project resolution — silent on
# success.

# step 2: acquire the driver lock.
#   specify change plan lock acquire --pid <agent-session-pid>
#     → Error::DriverBusy { pid: 48217 }
#
# The CLI's `specify change plan lock acquire` verb does the full
# liveness check (is PID 48217 alive? is .specify/plan.lock a
# sensible PID file?) before returning DriverBusy. This skill does
# not re-check.

/change:execute: driver already running.
  holder PID: 48217
  lock file: .specify/plan.lock

Nothing was changed. Wait for the running driver to finish, or
if you believe PID 48217 is stale, re-run `specify change plan lock
status` to confirm and then retry.

Exit 1
```

## No plan changes

`plan.yaml` is byte-identical before and after the refused run. No self-heal runs (step 3 is never reached). No iteration runs (step 4 is never reached). No terminal summary is emitted — the refused run did not cross into the protected region that would need summarising. The whole run is strictly read-only against the plan.

## Invariants pinned

1. **Lock refusal happens before self-heal.** Step 2 is the very first write-observable operation, and it is the operation that fails. Self-heal (step 3) is only reached once the lock is held; therefore a DriverBusy refusal never runs self-heal, never reads `.metadata.yaml`, and never writes to `journal.yaml`.
2. **No terminal summary on refusal.** The terminal summary is a `--loop`-scoped emission bookending the iteration body. A run that never enters the iteration body also never bookends it. The operator sees the conflict diagnostic and an exit code; the `Completion:` classifications (`all-done` / `stuck` / `halted` / `driver-interrupted`) do not apply.
3. **Holder PID reported verbatim.** The `pid` value in `Error::DriverBusy { pid: 48217 }` is rendered as-is into the operator-facing diagnostic. A human verifying the conflict runs `ps -p 48217` and matches it against what the CLI reports.
4. **Exit code 1.** The refused run exits non-zero so CI / scripting can distinguish "driver ran and did nothing because the plan was already all-done" (exit 0 with `Completion: all-done`) from "driver never ran because another driver was already running" (exit 1 with DriverBusy).
5. **Lock release not run.** The refused invocation never acquired the lock; it must not release it either. Running `specify change plan lock release` on a lock held by another PID would be an authentication failure (the release verb checks `--pid` against the stamp).

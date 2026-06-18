# Plan lock — CLI contract

The plan lock is an OS-level exclusive advisory file lock taken on `.specify/plan.lock` (or `<workspace>/.specify/plan.lock` in workspace mode). The lock identity is the file lock itself; the file body carries the holder pid, hostname, and acquisition timestamp purely as diagnostic noise.

Acquisition is a CLI verb. `specify plan lock -- <cmd>` is the `flock(1)`-style command-wrapper: the CLI takes the lock, runs `<cmd>` under it, and releases the lock when `<cmd>` exits. The child's exit code is passed through unchanged. Acquisition is non-blocking: a second `/spec:execute` (or a `/spec:refine` / `/spec:build` / `/spec:merge` breakout) that finds the lock held exits immediately with the structured error `plan-lock-busy` (exit 2) and the holder pid.

There is no separate `plan lock {acquire,release,status}` verb family — the single command-wrapper is the whole surface. The CLI owns acquisition cross-platform (it takes the lock with the same advisory-lock primitive on Linux and macOS), so there is no shell snippet to copy and **no `flock(1)` / `zsystem` / Python `fcntl` fallback** to maintain.

## The CLI also probes the lock

Acquisition and enforcement are both CLI-owned: the plan-state-writing verbs — `specify plan next`, per-entry `specify plan transition` (including `--undo`), and a plan-backed `specify slice merge run` — probe the lock before writing and refuse an unlocked driver with the structured error `plan-lock-not-held` (exit 2). Because those verbs run as children of `specify plan lock -- <cmd>`, they observe the parent-held lock and pass; a session that drove plan state without the wrapper cannot advance, close, or merge a plan entry. Exemptions: the plan-level Gate 1 stamp (`specify plan transition <plan-name> approved` precedes any driver session) and standalone merges in plan-less fixtures. The probe resolves the lock at the plan root (`--plan-dir` / `SPECIFY_PLAN_DIR`), so slot-side merge work probes the *workspace* lock. Proven by the named CLI test [`tests/workflow/plan_lock.rs`](https://github.com/augentic/specify/blob/main/engine/tests/workflow/plan_lock.rs).

## Usage

Wrap the work that drives plan state in the verb:

```bash
specify plan lock -- <command>
```

`<command>` is everything after the `--` separator, run verbatim as a child process with stdio inherited. Pass any global flags (e.g. `--plan-dir`) before the separator:

```bash
specify --plan-dir "$WORKSPACE_ROOT" plan lock -- <command>
```

The `/spec:execute` loop wraps its whole iteration in the verb; a standalone breakout wraps its single phase invocation.

On contention the verb prints the structured error and exits 2 before spawning the child:

```text
error: plan-lock-busy: another driver session holds the plan lock: holder-pid=12345
```

## Re-entrancy

The wrapper exports `SPECIFY_PLAN_LOCK_HELD=1` into the child's environment. A nested `specify plan lock -- <cmd>` — a breakout phase spawned under a parent `/spec:execute` that already holds the lock — sees the variable, **skips re-acquisition**, and just runs its command. Skills no longer need to read or propagate the variable themselves; the CLI owns the re-entrant handshake.

## Release semantics

- **Child exit releases the lock.** The lock lives exactly as long as the `<command>` the wrapper spawned; when it exits (cleanly, on error, or on signal), `specify` closes the descriptor and the OS releases the lock.
- **Stale lockfile.** If the holder process died without releasing (`kill -9`, OOM, host crash), the OS file lock is gone but the lockfile body remains. The next acquire succeeds because the lock is unheld; the body is overwritten with the new holder.
- **No watchdog, no liveness probe.** There is no auto-recovery for a lock whose holder process is permanently wedged. The operator runs `kill -0 <holder-pid>` to confirm the holder is dead, then `rm .specify/plan.lock`.

## Diagnostic output

The structured error printed on `plan-lock-busy` carries the holder pid:

```text
plan-lock-busy holder-pid=12345
```

`/spec:execute` and the breakout skills surface that line and exit non-zero. No retry loop, no prompt.

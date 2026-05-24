# Plan lock — env-var detection and acquisition

Shared reference for the three skills that participate in the plan lock: `/spec:execute`, `/spec:build`, `/spec:merge`. The lock prevents two `/spec:execute` runs (or an execute-plus-breakout pair) from racing on the same plan.

## Lock identity

- **Path** — `.specify/plan.lock` (single-repo) or `<workspace-root>/.specify/plan.lock` (workspace mode). The driver picks the workspace-root path whenever `.specify/project.yaml` carries `workspace: true`.
- **Mechanism** — exclusive advisory file lock acquired with `flock(LOCK_EX | LOCK_NB)` on POSIX, `LockFileEx` on Windows. Released on process exit.
- **Body** — pid + hostname + acquisition timestamp. Diagnostics only; the lock identity is the file lock itself, not the body.

## Re-entrancy heuristic — `SPECIFY_PLAN_LOCK_HELD=1`

The breakout skills (`/spec:build`, `/spec:merge`) share their bodies with the `/spec:execute` loop. When the loop calls into them, the parent already holds the lock; a re-entrant `flock(LOCK_EX | LOCK_NB)` from the child would fail with `EWOULDBLOCK` and abort the loop.

The detection contract is a single environment variable:

- `/spec:execute` MUST export `SPECIFY_PLAN_LOCK_HELD=1` after acquiring the lock and before invoking any breakout body. The variable must remain set for every nested invocation in the loop.
- `/spec:build` and `/spec:merge` MUST check `$SPECIFY_PLAN_LOCK_HELD` at entry. When equal to `"1"`, do **not** acquire the lock — the parent owns it. Otherwise (standalone invocation), acquire the lock at the path above.
- The variable is the only signal. There is no fallback to fcntl-aware re-entrancy in v1; the env var is the lighter-touch choice and aligns the three bodies on a single observable contract.

## Acquisition snippet

The standalone path acquires, holds for the duration of the body, and releases on process exit:

```bash
PLAN_LOCK="${SPECIFY_PLAN_LOCK_HELD:-0}"
if [ "$PLAN_LOCK" = "1" ]; then
    : "plan lock held by parent (/spec:execute); skipping acquire"
else
    LOCK_PATH=".specify/plan.lock"
    if grep -q '^workspace: true' .specify/project.yaml 2>/dev/null; then
        LOCK_PATH="$(git rev-parse --show-toplevel)/.specify/plan.lock"
    fi
    exec 9>"$LOCK_PATH"
    if ! flock -n 9; then
        echo "plan-lock-busy: another /spec:execute or breakout holds $LOCK_PATH" >&2
        exit 1
    fi
    printf 'pid=%d host=%s acquired=%s\n' "$$" "$(hostname)" "$(date -Iseconds)" >&9
    export SPECIFY_PLAN_LOCK_HELD=1
fi
```

The lock releases when fd 9 closes at process exit; no explicit `flock -u` call is required.

## Stale lockfile

A second invocation that finds the lock held exits immediately with `plan-lock-busy` and the holder's pid. The operator either waits, or — if the holder is dead — removes the lockfile by hand. v1 has no automatic stale-lock detection.

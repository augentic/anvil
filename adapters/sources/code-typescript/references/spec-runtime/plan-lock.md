# Plan lock — shell contract

The plan lock is an OS-level exclusive advisory file lock taken on `.specify/plan.lock` (or `<workspace>/.specify/plan.lock` in workspace mode). The lock identity is the file lock itself; the file body carries the holder pid, hostname, and acquisition timestamp purely as diagnostic noise. Acquisition is non-blocking: a second `/spec:execute` (or a `/spec:refine` / `/spec:build` / `/spec:merge` breakout) that finds the lock held exits immediately with the structured error `plan-lock-busy` and the holder pid.

There is no `specify plan lock {acquire,release,status}` CLI verb — the lock is the `flock`-based snippet below, never a CLI surface (the `cli-contract.md` verb tree records the same `.specify/plan.lock` snippet as the driver lock, "not a CLI verb"). Every skill that touches plan state from outside the loop — `/spec:execute` itself, plus the three breakout skills when invoked standalone — reuses the snippet below verbatim.

## Primary path — `flock -n`

Linux ships `flock(1)` out of the box; macOS via Homebrew's `util-linux` package (`brew install util-linux`) does the same. The acquiring shell holds the lock for the duration of the bash session that opens fd 9; closing the shell auto-releases it.

```bash
mkdir -p .specify
LOCK=.specify/plan.lock
exec 9>>"$LOCK"
if ! flock -n 9; then
  holder=$(awk -F= '/^pid=/{print $2}' "$LOCK" 2>/dev/null || true)
  printf 'plan-lock-busy holder-pid=%s\n' "${holder:-unknown}" >&2
  exit 1
fi
printf 'pid=%s\nhostname=%s\nacquired-at=%sZ\n' \
  "$$" "$(hostname)" "$(date -u +%FT%T)" >&9
trap 'rm -f "$LOCK"' EXIT
```

The `trap` is courtesy cleanup; the lock is released whether or not `rm` runs.

## Fallback path — macOS without `util-linux`

On a stock macOS the bare `flock` binary is absent. Detect with `command -v flock`; if missing, drive the same `LOCK_EX | LOCK_NB` call through Python's `fcntl`. The Python interpreter holds the lock for its lifetime, so the surrounding shell must run the loop body inside the same `python3` invocation (use `subprocess` from Python, or `os.execvp` back into bash after the lock is acquired, depending on the caller's preference).

```bash
if ! command -v flock >/dev/null 2>&1; then
  python3 - <<'PY' || exit 1
import fcntl, os, socket, sys
from datetime import datetime, timezone
path = ".specify/plan.lock"
fd = os.open(path, os.O_CREAT | os.O_RDWR, 0o644)
try:
    fcntl.flock(fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
except BlockingIOError:
    holder = ""
    try:
        with open(path) as f:
            for line in f:
                if line.startswith("pid="):
                    holder = line.strip().split("=", 1)[1]
    except FileNotFoundError:
        pass
    print(f"plan-lock-busy holder-pid={holder or 'unknown'}", file=sys.stderr)
    sys.exit(1)
os.ftruncate(fd, 0)
body = (
    f"pid={os.getpid()}\n"
    f"hostname={socket.gethostname()}\n"
    f"acquired-at={datetime.now(timezone.utc).isoformat()}\n"
).encode()
os.write(fd, body)
# Hand off to the loop body here (e.g. os.execvp("bash", [...])); the lock
# is held until this Python process exits.
PY
fi
```

## Release semantics

- **Process exit** releases the lock. The shell that ran `flock -n 9` or the `python3` interpreter that called `fcntl.flock` is the holder.
- **Stale lockfile.** If the holder process died without releasing (`kill -9`, OOM, host crash), the OS file lock is gone but the lockfile body remains. The next acquire succeeds because the lock is unheld; the body is overwritten with the new holder.
- **No watchdog, no liveness probe.** There is no auto-recovery for an `flock`-held lock whose holder process is permanently wedged. The operator runs `kill -0 <holder-pid>` to confirm the holder is dead, then `rm .specify/plan.lock`.

## Diagnostic output

The structured error printed on `plan-lock-busy` is intentionally one line and parseable:

```text
plan-lock-busy holder-pid=12345
```

`/spec:execute` and the breakout skills surface that line on stderr verbatim and exit non-zero. No JSON envelope, no retry loop, no prompt.

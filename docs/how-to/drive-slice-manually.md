# Drive a slice manually

Resume or run one slice phase when `/spec:execute` parks or when you want operator control.

**Prerequisites:** A `reviewed` plan with at least one slice entry; completed [Quick start](../tutorials/quick-start.md).

## When execute parks on build failure

1. Read the stop hint — note `failing-task` and `log-path`.
2. Fix the code or configuration issue.
3. Re-run build for the active slice:

```text
/spec:build <slice-name>
```

4. If build succeeds, either merge by hand or resume execute:

```text
/spec:merge <slice-name>
```

—or—

```text
/spec:execute
```

Execute re-enters at the active `in-progress` entry and skips phases already complete.

## When execute parks on merge conflict

1. Read conflicting baseline paths from the stop hint.
2. Re-run refine against the current baseline, or hand-edit slice specs.
3. Retry merge:

```text
/spec:merge <slice-name>
```

## Breakout mid-execute

Cancel a running `/spec:execute` session and drive phases yourself:

```text
/spec:build <slice-name>
/spec:merge <slice-name>
/spec:execute
```

The execute loop reads on-disk lifecycle state — no resume flags required.

## Plan lock

Standalone breakouts acquire `.specify/plan.lock` the same way execute does. If you see `plan-lock-busy`, another process holds the lock. When the holder is dead, remove the stale lock file manually.

## See also

- [/spec:execute](../reference/change-skills/execute.md) — stop conditions and re-entry
- [Drop down a layer](drop-down-a-layer.md) — full manual Layer 2 control
- [Slice skills](../reference/slice-skills/index.md) — refine, build, merge reference

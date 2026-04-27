# Recover from a Failed Change

A change can fail during any phase -- define, build, or merge. The recovery path depends on whether you are running manually or through `/spec:execute`.

## Diagnose the state

Check what state the change is in:

```bash
specify change status <name>
```

| State | What happened | Next step |
|-------|--------------|-----------|
| `defining` | Define crashed mid-pipeline | Re-run `/spec:define` or drop and redefine |
| `defined` | Define completed but build hasn't started | Run `/spec:build` |
| `building` | Build crashed or a task failed | Resume `/spec:build` (it picks up from the last incomplete task) |
| `complete` | All tasks done but merge failed | Check the merge error and retry `/spec:merge` |

## Manual recovery

### Re-run the failed phase

The simplest fix is to re-run the phase that failed:

```text
/spec:build              # resumes from the last incomplete task
/spec:merge              # retries the merge
```

### Drop and redefine

If the artifacts are in a bad state, discard and start over:

```text
/spec:drop
/spec:define "same description, refined if needed"
```

### Fix artifacts manually

You can edit artifact files directly (e.g. fix a design issue in `design.md`) and then continue:

```text
/spec:build              # reads the updated artifacts
```

## Recovery during `/spec:execute`

When `/spec:execute` encounters a failure:

1. It drops the failed change automatically.
2. The plan entry transitions to `failed`.
3. In `--loop` mode, execution continues to the next eligible entry.

To retry the failed entry:

```bash
specify plan transition <name> pending
```

Then re-run `/spec:execute`. The driver will pick up the reset entry.

If the failure was caused by a dependency, you can also skip the entry:

```bash
specify plan transition <name> skipped
```

## Self-heal on restart

If `/spec:execute` was interrupted (crash, Ctrl+C), the next invocation performs self-heal automatically -- it detects the stale `in-progress` entry, inspects the change state, and transitions the plan entry to `done`, `failed`, or `blocked` as appropriate.

## See also

- [Troubleshooting](../appendices/troubleshooting.md) -- symptom/cause/fix index for common errors
- [/spec:drop](../reference/change-skills/drop.md) -- reference for discarding changes
- [/spec:execute](../reference/initiative-skills/execute.md) -- self-heal and failure handling details

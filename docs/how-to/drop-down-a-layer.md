# Drop down a layer

Run Emery manually when a higher automation layer fails.

**Prerequisites:** Familiarity with the [Quick start](../tutorials/quick-start.md) workflow.

Emery organises work in three layers above the CLI substrate. See [The layered stack](../explanation/layered-stack.md).

| Layer | What it automates | Manual fallback |
| ----- | ----------------- | --------------- |
| Layer 2 | `/emery:plan`, `emery plan execute`, `/emery:finalize` | Individual skills + `emery plan *` / `emery slice *` |
| Layer 1 | Per-slice refine → build → merge inside execute | Run `/emery:refine`, `/emery:build`, `/emery:merge` by hand |
| Layer 0 | Project and adapter configuration | `emery init`, `emery source resolve`, `emery target resolve` |

## When execute fails

1. Read the stop hint from `emery plan execute` (failing task, log path, or conflict paths).
2. Fix the underlying issue in code or specs.
3. Resume with `emery plan execute` or run the parked phase manually — see [Drive a slice manually](drive-slice-manually.md).

## When you want full manual control

Drive one slice without the execute loop — the breakout verbs need no driver lock (mutual exclusion is guest-owned by the `plan execute` marker; the lifecycle gates fence breakouts). Hand-driven plans need no approve stamp — claiming the first entry is your Gate 1 decision:

```bash
emery plan next
# /emery:refine <slice>, /emery:build <slice>, /emery:merge <slice>
```

Repeat `emery plan next` between slices when the plan has multiple entries; `emery plan status` (read-only, never lock-gated) shows the next action at any point.

## When finalize is blocked

If publication or archive is blocked, complete publication through the repository's normal tooling, then archive directly:

```bash
git push -u origin HEAD
emery plan archive <name>
```

See [emery plan](../reference/cli/plan.md) and [Workspace topology](../reference/cli/workspace.md).

## See also

- [Drive a slice manually](drive-slice-manually.md) — breakout after execute parks
- [Lifecycle](../reference/lifecycle.md) — legal transitions the CLI enforces

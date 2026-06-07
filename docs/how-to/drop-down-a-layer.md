# Drop down a layer

Run Specify manually when a higher automation layer fails.

**Prerequisites:** Familiarity with the [Quick start](../tutorials/quick-start.md) workflow.

Specify organises work in three layers above the CLI substrate. See [The layered stack](../explanation/layered-stack.md).

| Layer | What it automates | Manual fallback |
| ----- | ----------------- | --------------- |
| Layer 2 | `/spec:plan`, `/spec:execute`, `/spec:finalize` | Individual skills + `specify plan *` / `specify slice *` |
| Layer 1 | Per-slice refine → build → merge inside execute | Run `/spec:refine`, `/spec:build`, `/spec:merge` by hand |
| Layer 0 | Project and adapter configuration | `specify init`, `specify source resolve`, `specify target resolve` |

## When execute fails

1. Read the stop hint from `/spec:execute` (failing task, log path, or conflict paths).
2. Fix the underlying issue in code or specs.
3. Resume with `/spec:execute` or run the parked phase manually — see [Drive a slice manually](drive-slice-manually.md).

## When you want full manual control

Drive one slice without the execute loop:

```bash
specify plan transition <name> approved
specify plan next
/spec:refine <slice>
/spec:build <slice>
/spec:merge <slice>
```

Repeat `specify plan next` between slices when the plan has multiple entries.

## When finalize is blocked

If push or PR observation fails, use CLI verbs directly:

```bash
specify workspace push
specify plan archive <name>
```

See [specify plan](../reference/cli/plan.md) and [specify workspace](../reference/cli/workspace.md).

## See also

- [Drive a slice manually](drive-slice-manually.md) — breakout after execute parks
- [Lifecycle](../reference/lifecycle.md) — legal transitions the CLI enforces

# Drop down a layer

Run Emery manually when a higher automation layer fails.

**Prerequisites:** Familiarity with the [Quick start](../tutorials/quick-start.md) workflow.

Emery organises work in three layers above the CLI substrate. See [The layered stack](../explanation/layered-stack.md).

| Layer | What it automates | Manual fallback |
| ----- | ----------------- | --------------- |
| Layer 2 | `/emery:plan`, `emery plan execute`, `/emery:finalize` | Individual skills + the `emery plan *` verbs |
| Layer 1 | Per-slice refine → build → merge inside execute | None — fix inputs, then re-run `emery plan execute` |
| Layer 0 | Project and adapter configuration | `emery init`, `emery source resolve`, `emery target resolve` |

## When execute fails

1. Read the stop hint from `emery plan execute` (failing task, log path, or conflict paths).
2. Fix the underlying issue in code or specs. Plan curation stays on the CLI: `emery plan add` / `amend` / `remove` / `drop`.
3. Re-run `emery plan execute` — the loop resumes at the [parked](../appendices/glossary.md#p) phase. `emery plan status` (read-only, never lock-gated) shows the next action at any point.

There are no per-phase operator verbs: the refine, build, and merge phases run only inside the execute loop, and re-running execute is always the resume path. Amending a slice's inputs drifts its pins, so the next execute re-refines exactly the affected slices.

## When finalize is blocked

If publication or archive is blocked, complete publication through the repository's normal tooling, then archive directly:

```bash
git push -u origin HEAD
emery plan archive
```

See [emery plan](../reference/cli/plan.md).

## See also

- [Recover from a stale guest lock](recover-from-a-stale-guest-lock.md) — when execute refuses with `guest-marker-held`
- [Lifecycle](../reference/lifecycle.md) — legal transitions the CLI enforces

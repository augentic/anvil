# Drop down a layer

Run Emery manually when a higher automation layer fails.

**Prerequisites:** Familiarity with the [Quick start](../tutorials/quick-start.md) workflow.

Emery organises work in three layers above the CLI substrate. See [The layered stack](../explanation/layered-stack.md).

| Layer | What it automates | Manual fallback |
| ----- | ----------------- | --------------- |
| Layer 2 | `/emery:plan`, `emery plan refine`, `emery plan execute`, `/emery:finalize` | Individual skills + the `emery plan *` verbs |
| Layer 1 | Per-slice refinement inside `plan refine`; build → merge inside execute | None — fix inputs, then re-run the parked stage |
| Layer 0 | Project and adapter configuration | `emery init`, `emery source resolve`, `emery target resolve` |

## When refine or execute fails

1. Read the stop hint from the stopped command (failing slice, failing task, log path, or conflict paths).
2. Fix the underlying issue in inputs or specs. Plan curation stays on the CLI: `emery plan add` / `amend` / `remove` / `drop`.
3. Re-run the same command — `emery plan refine` skips fresh manifests and resumes the missing or stale work; `emery plan execute` resumes at the [parked](../appendices/glossary.md#p) phase. `emery plan status` (read-only, never lock-gated) shows the next action at any point.

There are no per-slice phase-breakout verbs: refinement runs only inside the `plan refine` drain and the build and merge phases only inside the execute loop. Amending a slice's inputs stales its refinement manifest, so the next `emery plan refine` re-refines exactly the affected slices — execute never refines and refuses missing or stale manifests with `plan-refinement-required`.

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

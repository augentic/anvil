# /spec:execute

Drive a reviewed plan through refine → build → merge per entry under an exclusive plan lock.

## Synopsis

```text
/spec:execute
```

Takes no positional arguments and no flags. The active plan is the one at `.specify/plan.yaml`.

## When to use

- Gate 1 has stamped the plan `reviewed` and you want to drive every slice to completion.
- Re-entering after execute parks on a build failure or merge conflict (reads on-disk state; no resume flags).

Not before Gate 1, nor after every per-entry status is `done` (use [/spec:finalize](finalize.md)).

## Artifacts read/written

| Artifact | Role |
| -------- | ---- |
| `.specify/plan.yaml` | Reads lifecycle and per-entry status; never writes `reviewed` or `done` directly |
| `.specify/plan.lock` | Exclusive advisory lock for the duration of the loop |
| Slice directories | Created and updated by phase skills (`/spec:refine`, `/spec:build`, `/spec:merge`) |
| Per-entry `done` | Written only by `/spec:merge` via `specify slice merge` |

## Behavior

1. **Refusal gate** — `specify plan next --format json` refuses when `plan-not-reviewed`; prints `specify plan transition <name> reviewed` verbatim.
2. **Acquire plan lock** — exclusive non-blocking lock on `.specify/plan.lock` (workspace root in workspace mode). On `plan-lock-busy`, exit with holder pid.
3. **Loop** — for each `specify plan next` result:
   - Route to workspace slot when `project` is set.
   - Invoke `/spec:refine` when slice is fresh (`refining` or absent).
   - Invoke `/spec:build` when slice is `refined`.
   - Invoke `/spec:merge` when slice is `built`.
4. **Stop on first failure** — build non-zero exit or merge baseline conflict leaves entry `in-progress`; surface stop hint.
5. **Drain** — when no `pending` or `in-progress` entries remain, print `drained — run /spec:finalize <name>` and release lock.

Re-entry is implicit: re-running `/spec:execute` picks up the active `in-progress` entry and resumes mid-loop.

### Workspace routing

When a plan entry carries `project`, plan artifacts stay at the workspace root and phase work runs in `.specify/workspace/<project>/`. See [specify workspace](../cli/workspace.md).

## Lifecycle interactions

| Trigger | Transition | Writer |
| ------- | ---------- | ------ |
| `specify plan next` picks pending row | per-entry: `pending → in-progress` | CLI |
| `/spec:merge` succeeds | per-entry: `in-progress → done` | `specify slice merge` |

Execute never writes `reviewed` or `done` directly.

## Error modes

| Error | Cause | Resolution |
| ----- | ----- | ---------- |
| `plan-not-reviewed` | Plan still `pending` | Run `specify plan transition <name> reviewed` |
| `plan-lock-busy` | Another process holds `.specify/plan.lock` | Wait or remove stale lock if holder is dead |
| Build failure | Task exited non-zero | Fix failure; re-run `/spec:execute` or [/spec:build](../slice-skills/build.md) |
| Merge conflict | Baseline drift | Resolve conflict; re-run execute or merge |

## Examples

```text
# Drive every slice after Gate 1
specify plan transition fix-typo reviewed
/spec:execute
```

## See also

- [Drive a slice manually](../../how-to/drive-slice-manually.md) — when execute parks
- [Drop down a layer](../../how-to/drop-down-a-layer.md) — manual CLI fallback
- [/spec:finalize](finalize.md) — post-drain closure
- [Slice skills](../slice-skills/index.md) — refine, build, merge breakouts

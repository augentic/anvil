# /spec:execute

Drive an initiative through its plan, automating define-build-merge.

## Synopsis

```text
/spec:execute              # run one change, stop
/spec:execute --dry-run    # preview next change + progress
/spec:execute --loop       # run until no eligible change remains
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `--dry-run` | No | Preview what would happen without making changes |
| `--loop` | No | Run continuously until all changes are `done` or execution is `stuck` |

## When to use

- A `plan.yaml` exists and you want to automate the change-by-change execution loop.
- You prefer automated execution over manually invoking define/build/merge for each change.

## Artifacts produced

None of its own. Invokes `/spec:define`, `/spec:build`, `/spec:merge` (and `/spec:drop` on failure) for each change. Writes plan entry transitions via `specify plan transition`. Manages `.specify/plan.lock` for concurrency safety.

## Behavior

### Per-change algorithm

1. **Pick next.** `specify plan next --format json` returns the first `pending` entry whose `depends-on` are all `done`. The JSON response includes `project`, `description`, and `sources` for the entry.
2. **Lock.** Acquires `.specify/plan.lock` via `specify plan lock acquire`.
3. **Self-heal.** On startup, checks for stale `in-progress` entries from a prior crashed run and resolves them. For entries with `project`, metadata is read under the target project's workspace clone, not the initiating repo.
4. **Transition.** Moves the entry to `in-progress` via `specify plan transition`.
5. **CWD routing (multi-repo only).** If `project` is non-null: resolve the target directory from `registry.yaml`, check workspace freshness via `specify workspace status` (halt if `missing`), save CWD, resolve source paths to absolute paths, and `chdir` into the target project root. Emits `Routing: <name> → <project> (<path>)`.
6. **Define.** Invokes `/spec:define` with the entry's description and resolved sources.
7. **Build.** Invokes `/spec:build`.
8. **Merge.** Invokes `/spec:merge`. In workspace clones, the CLI auto-commits `.specify/` subtrees with message `specify: merge <change-name>`.
9. **CWD restore (multi-repo only).** Restores CWD to the initiating repo root.
10. **Read outcome.** Reads the phase outcome from `.metadata.yaml`.
11. **Transition plan entry:**
   - `success` --> `done`
   - `failure` --> `failed` (invokes `/spec:drop` first)
   - `deferred` --> `blocked`
12. **Release lock.**

### Loop mode

With `--loop`, the algorithm repeats from step 1 until:

- **`all-done`** -- every entry is `done`, `skipped`, or `failed`.
- **`stuck`** -- no `pending` entry has all dependencies satisfied.
- **SIGINT/SIGTERM** -- graceful shutdown after the current change completes.

### Dry-run mode

With `--dry-run`, reports the next eligible change and current plan progress without executing anything.

## Lifecycle transitions

Transitions plan entries: `pending --> in-progress --> done|failed|blocked`.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| No plan exists | `plan.yaml` not found | Run `/spec:plan` first |
| Lock held | Another `/spec:execute` session is running | Wait for it to finish or release the lock |
| Self-heal failure | Stale `in-progress` entry cannot be resolved | Manually transition or drop the stale change |
| Stuck | No eligible entries remain but not all are done | Review `failed`/`blocked` entries and resolve |

## Examples

```text
# Preview what would happen next
/spec:execute --dry-run

# Run one change
/spec:execute

# Run until all done
/spec:execute --loop
```

**Typical initiative flow:**

```text
/spec:plan migrate-to-v2 --source monolith=/path/to/legacy
specify plan status                # review the plan
/spec:execute --loop               # run until all-done
```

## See also

- [/spec:plan](plan.md) -- author the plan that execute consumes
- [/spec:define](../change-skills/define.md), [/spec:build](../change-skills/build.md), [/spec:merge](../change-skills/merge.md) -- the skills invoked per change
- [Lifecycle](../lifecycle.md) -- plan entry states
- [Troubleshooting](../../appendices/troubleshooting.md) -- self-heal, lock issues

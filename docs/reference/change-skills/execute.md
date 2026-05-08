# /change:execute

Drive a change through its plan, automating define-build-merge.

> **Renamed.** This skill was previously `/change:execute`. RFC-13 §3.9 moved it to the `change` plugin as `/change:execute`. The historical command remains as a deprecation shim that delegates here and is removed before the post-RFC-13 release; see [RFC-13 §Migration](../../../rfcs/archive/rfc-13-extensibility.md#migration).

## Synopsis

```text
/change:execute              # run one slice, stop
/change:execute dry-run    # preview next slice + progress
/change:execute loop       # run until no eligible slice remains
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `--dry-run` | No | Preview what would happen without making changes |
| `--loop` | No | Run continuously until all slices are `done` or execution is `stuck` |

## When to use

- A `plan.yaml` exists and you want to automate the slice-by-slice execution loop.
- You prefer automated execution over manually invoking define/build/merge for each slice.

## Artifacts produced

For routed workspace entries, prepares the selected workspace branch before phase writes and may create a non-baseline residue commit after merge. It invokes `/spec:define`, `/spec:build`, `/spec:merge` (and `/spec:drop` on failure) for each slice. Writes plan entry transitions via `specify change plan transition`. Manages `.specify/plan.lock` for concurrency safety.

## Behavior

### Per-slice algorithm

1. **Resolve project.** Walk upward from CWD looking for `.specify/project.yaml`.
2. **Lock.** Acquires `.specify/plan.lock` via `specify change plan lock acquire`.
3. **Self-heal.** Checks for stale `in-progress` entries from a prior crashed run and resolves them. For entries with `project`, metadata is read under the target project's workspace clone, not the initiating repo.
4. **Pick next.** `specify change plan next --format json` returns the first `pending` entry whose `depends-on` are all `done`. The JSON response includes `project`, `description`, and `sources` for the entry.
5. **Workspace preparation (multi-repo only).** If `project` is non-null: resolve the selected project through `registry.yaml`, materialise only that slot when missing, and run `specify workspace prepare-branch <project> --change <change-name>` before any phase writes.
6. **Transition and CWD routing.** Moves the entry to `in-progress` via `specify change plan transition`, saves CWD, resolves source paths to absolute paths, and `chdir`s into the prepared target project root. Emits `Routing: <name> → <project> (<path>)`.
7. **Define.** Invokes `/spec:define` with the entry's description and resolved sources.
8. **Build.** Invokes `/spec:build`.
9. **Merge.** Invokes `/spec:merge`. In workspace clones, the CLI auto-commits only `.specify/specs/` and `.specify/archive/` with message `specify: merge <slice-name>`.
10. **Residue guard (multi-repo merge success only).** Verifies the baseline commit boundary is clean and commits remaining non-baseline residue as `specify: residue <slice-name>` before `done`.
11. **CWD restore (multi-repo only).** Restores CWD to the initiating repo root.
12. **Read outcome.** Reads the phase outcome from `.metadata.yaml`.
13. **Transition plan entry:**
    - `success` --> `done`
    - `failure` --> `failed` (invokes `/spec:drop` first)
    - `deferred` --> `blocked`
14. **Release lock.**

### Loop mode

With `--loop`, the algorithm repeats from step 1 until:

- **`all-done`** -- every entry is `done` or `skipped`.
- **`stuck`** -- no `pending` entry has all dependencies satisfied.
- **SIGINT/SIGTERM** -- graceful shutdown after the current slice completes.

### Dry-run mode

With `--dry-run`, reports the next eligible slice and current plan progress without executing anything.

## Lifecycle transitions

Transitions plan entries: `pending --> in-progress --> done|failed|blocked`.

## Error modes

| Error | Cause | Resolution |
|-------|-------|------------|
| No plan exists | `plan.yaml` not found | Run `/change:plan` first |
| Lock held | Another `/change:execute` session is running | Wait for it to finish or release the lock |
| Self-heal failure | Stale `in-progress` entry cannot be resolved | Manually transition or drop the stale slice |
| Stuck | No eligible entries remain but not all are done | Review `failed`/`blocked` entries and resolve |

## Examples

```text
# Preview what would happen next
/change:execute dry-run

# Run one slice
/change:execute

# Run until all done
/change:execute loop
```

**Typical change flow:**

```text
/change:plan migrate-to-v2 source monolith=/path/to/legacy
specify change plan status         # review the plan
/change:execute loop             # run until all-done
```

## See also

- [/change:plan](plan.md) -- author the plan that execute consumes
- [/spec:define](../slice-skills/define.md), [/spec:build](../slice-skills/build.md), [/spec:merge](../slice-skills/merge.md) -- the skills invoked per slice
- [Lifecycle](../lifecycle.md) -- plan entry states
- [Troubleshooting](../../appendices/troubleshooting.md) -- self-heal, lock issues

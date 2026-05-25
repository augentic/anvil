# /spec:finalize

Close a drained change: push branches, observe PR state until every PR is `MERGED`, then archive the plan.

## Synopsis

```text
/spec:finalize <name>
```

## Arguments

| Argument | Required | Description |
| -------- | -------- | ----------- |
| `<name>` | Yes | Plan name matching `plan.yaml.name`. |

## When to use

- Every per-entry plan status is `done` and the operator is ready to close the change.
- After `/spec:execute` prints `drained — run /spec:finalize <name>`.

Not for plan authoring ([/spec:plan](plan.md)) or per-slice execution ([/spec:execute](execute.md)).

## Artifacts read/written

The skill writes nothing under `.specify/` directly. Every state mutation is a CLI shell-out:

| Step | CLI / tool | Effect |
| ---- | ---------- | ------ |
| Drain check | `specrun plan next` | Confirms `reason: drained` |
| Push | `specrun workspace push` | Publishes `specify/<name>` branches as PRs |
| Observe | `gh pr view` | Polls until each PR is `MERGED` |
| Archive | `specrun plan archive` | Moves plan to `.specify/archive/plans/` |

PR merges stay operator-owned — finalize observes state; it does not call `gh pr merge`.

## Behavior

1. **Pre-flight** — validate `<name>`; verify active `plan.yaml`.
2. **Drainage** — `specrun plan next --format json`; only `reason: drained` may continue.
3. **Push** — `specrun workspace push`; surface per-project status table.
4. **Observe PRs** — poll each pushed PR until state is `MERGED`.
5. **Archive** — `specrun plan archive`; print merged PRs, archive path, and post-merge tidy-ups.

### Closing message

```text
Change <name> finalized. Plan archived at <.specify>/archive/plans/<name>-<YYYYMMDD>.yaml.
```

Single-repo projects skip workspace push when no registry slots are involved; archive still runs once drainage is confirmed.

## Error modes

| Error | Cause | Resolution |
| ----- | ----- | ---------- |
| Plan not drained | Entries still `pending` or `in-progress` | Resume `/spec:execute` |
| Push failure | Remote or auth error | Fix remote access; re-run finalize |
| PR not merged | Operator has not merged PR | Merge PRs manually; re-run finalize |

## Examples

```text
# After execute drains
/spec:finalize fix-typo
```

## See also

- [/spec:execute](execute.md) — drives slices until drain
- [Cross-repo changes tutorial](../../tutorials/cross-repo-change.md) — workspace push and PR flow
- [specrun plan](../cli/plan.md) — `plan archive` and `plan finalize`
- [Registry](../registry.md) — multi-repo platform setup

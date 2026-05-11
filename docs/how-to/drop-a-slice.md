# Drop a slice

Use this page when a slice should not be merged -- it was exploratory, superseded, or just wrong -- and you want to archive it without touching the baseline.

## Prerequisites

- An active slice that you no longer want to merge.
- The slice's name (run `specify status` if you need to look it up).

## 1. Run `/spec:drop`

In the Cursor agent chat:

```text
/spec:drop
```

The skill prompts for confirmation, then archives the slice.

<details>
<summary>Expected output</summary>

```text
Drop slice add-greeting-endpoint? (y/n): y

Slice dropped.
  Archived: .specify/archive/2026-04-27-add-greeting-endpoint/
  Baseline unchanged.
```

</details>

## 2. (Optional) Skip the prompt with a reason

Pass `reason "<text>"` to record why the slice was dropped and bypass the interactive confirmation:

```text
/spec:drop reason "Superseded by a different approach"
```

The reason is stored on the archived slice's metadata.

## Verify

| Check | Command | Expect |
|-------|---------|--------|
| Slice no longer active | `specify status` | The dropped slice is absent from the active list. |
| Slice is archived with status `dropped` | `ls .specify/archive/` | A directory like `<date>-<slice-name>/` whose `.metadata.yaml` records `status: dropped`. |
| Baseline is unchanged | `git status .specify/specs/` | No modifications under `.specify/specs/`. |

## See also

- [Recover from a failed change](recover-failed-change.md) -- when the build or merge halted but the slice should still land.
- [`/spec:drop` reference](../reference/slice-skills/drop.md) -- full skill reference.

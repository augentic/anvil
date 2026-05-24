# /spec:build

Implement tasks from a refined slice by loading the target adapter's build brief.

## Synopsis

```text
/spec:build [slice-name]
```

## Arguments

| Argument | Required | Description |
| -------- | -------- | ----------- |
| `slice-name` | No | Name of the slice to build. When omitted, uses the active `in-progress` entry from `specify plan next`. Must match the active entry when supplied. |

## When to use

- A slice is `refined` and you want to start or continue implementation.
- `/spec:execute` parked on a build failure and you fixed the failing task.
- Running build standalone after `/spec:refine` outside the execute loop.

Not when the slice has not been refined (use [/spec:refine](refine.md)) or has already merged.

## Artifacts produced

Source code changes in the project codebase (not under `.specify/`). Task checkboxes in `tasks.md` are flipped via `specify slice task mark` as each task completes.

## Behavior

1. **Resolve active slice** — `specify plan next --format json`; refuse if `[slice-name]` mismatches active entry.
2. **Acquire plan lock** when invoked standalone (skip when `SPECIFY_PLAN_LOCK_HELD=1` from `/spec:execute`).
3. **Workspace routing** — `chdir` into `.specify/workspace/<project>/` when in workspace mode.
4. **Refuse on lifecycle** — proceed only when slice status is `refined`.
5. **Load target build brief** — `specify target resolve` + read `briefs/build.md`; follow orchestration linearly.
6. **Stop on failure** — non-zero exit leaves slice at `refined`; emit structured stop hint with failing task and log path.
7. **Transition on success** — `specify slice transition <name> built`.

Synthesis review tags in `spec.md` are not build blockers — build proceeds against whatever spec is on disk.

### Contract-only changes

The contracts adapter build brief dispatches to format sub-flows (`openapi`, `asyncapi`, `json-schema`), runs author or importer intent, then verifier intent with a verify-repair loop. No implementation code is generated.

## Lifecycle transitions

`refined → built` (stays `refined` on build failure)

## Error modes

| Error | Cause | Resolution |
| ----- | ----- | ---------- |
| Slice not refined | Lifecycle is `refining` or earlier | Run `/spec:refine` first |
| Lifecycle refused | Slice already `built`, `merged`, or `dropped` | Run appropriate next phase or drop |
| Build failure | Compile, test, or brief step exited non-zero | Fix failure; re-run `/spec:build` |
| Specialist skill failure | Delegated skill error | Fix and re-run build |

## Examples

```text
# Build the active in-progress slice
/spec:build

# Build a specific slice by hand
/spec:build fix-typo
```

## See also

- [/spec:refine](refine.md) — generate artifacts before building
- [/spec:merge](merge.md) — next step after all tasks complete
- [Drive a slice manually](../../how-to/drive-slice-manually.md) — when execute parks on build
- [Artifact format](../artifact-format.md) — skill directive tag syntax

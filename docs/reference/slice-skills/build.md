# /spec:build

Implement tasks from a refined slice by loading the target adapter's build brief.

## Synopsis

```text
/spec:build [slice-name]
```

## Arguments

| Argument | Required | Description |
| -------- | -------- | ----------- |
| `slice-name` | No | Name of the slice to build. When omitted, uses the active `in-progress` entry from `specrun plan next`. Must match the active entry when supplied. |

## When to use

- A slice is `refined` and you want to start or continue implementation.
- `/spec:execute` parked on a build failure and you fixed the failing task.
- Running build standalone after `/spec:refine` outside the execute loop.

Not when the slice has not been refined (use [/spec:refine](refine.md)) or has already merged.

## Artifacts produced

Source code changes in the project codebase (not under `.specify/`). The CLI writes the build request to `.specify/slices/<name>/build/request.yaml` in `--phase prepare`; the brief writes the build report to `.specify/slices/<name>/build/report.yaml`. Task checkboxes in `tasks.md` are flipped via `specrun slice task mark` as each task completes.

## Behavior

The authoritative step-by-step lives in the [`/spec:build` skill body](../../../plugins/spec/skills/build/SKILL.md); the operator summary follows. The skill drives the two-phase [`specrun slice build`](../cli/slice.md#specrun-slice-build) verb (prepare → brief → finalize), mirroring `specrun source survey` / `extract`. The CLI owns request assembly, report validation, the `target-build-*` aborts, the `slice.build.*` events, and the `built` transition gate; this skill owns only running the target build brief against the prepared request.

1. **Resolve active slice** — `specrun plan next --format json`; refuse if `[slice-name]` mismatches active entry.
2. **Acquire plan lock** when invoked standalone (skip when `SPECIFY_PLAN_LOCK_HELD=1` from `/spec:execute`).
3. **Workspace routing** — `chdir` into `.specify/workspace/<project>/` when in workspace mode.
4. **Refuse on lifecycle** — proceed only when slice status is `refined`.
5. **Prepare the build request** — `specrun slice build <name> --phase prepare --format json` resolves the target, assembles + schema-validates the request, emits `target.execution.agent`, and prints the handoff envelope (`slice`, `target`, `request`, `report`, `briefs-dir`, `build-brief`).
6. **Run the target build brief** — read the handoff's `build-brief` and execute it against the prepared request; agent codegen plus target-local validation. Write the build report to the handoff's `report` path (`status: success` clean, `status: failure` on a brief-side failure).
7. **Finalize and gate the transition** — `specrun slice build <name> --phase finalize --format json` frames with `slice.build.started`, validates the report, rejects a `success` report with any blocking finding, and on a clean success report **owns** the `refined → built` transition (`slice.build.succeeded`). On any failure it emits `slice.build.failed`, exits non-zero, and leaves the slice at `refined`. The skill never calls `specrun slice transition <name> built` by hand.

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

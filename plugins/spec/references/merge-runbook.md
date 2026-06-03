# Merge skill — runbook detail

The merge SKILL.md keeps the algorithmic spine; this file owns the verbatim
output templates and the workspace-clone commit semantics that step 5 invokes.

## Preview template

Render this from the `operations[]` array returned by
`specify slice merge preview --format json`. Operations are typed `added`,
`modified`, `removed`, `renamed`, or `created_baseline`:

```text
Merge Preview: <slice-name>

<adapter-1>/spec.md (existing baseline)
- REMOVING: REQ-001 — <name>
- MODIFYING: REQ-002 — <name>
- ADDING: REQ-003 — <name>

<adapter-2>/spec.md (new baseline)
- CREATING baseline with N requirements
```

If `specify slice merge preview` returns an empty `specs` array, report "No
delta specs to merge" and stop.

## Conflict-check surfacing

If `slice merge conflict-check` returns any entries under `conflicts`, surface
them clearly — each entry names the adapter, the slice's `defined-at`, and
the baseline's `baseline-modified-at`:

> "The baseline for `<adapter>` was modified at `<baseline-modified-at>`
> (after this slice was defined at `<defined-at>`). Another change may have
> already touched it."

## Workspace clone auto-commit

When CWD is inside a workspace clone (`.specify/workspace/*/` with
`.specify/project.yaml`), `specify slice merge run` auto-commits **only**
`.specify/specs/` and `.specify/archive/` with message
`specify: merge <slice-name>`. Commit failure is a **warning**, not an
error — the spec merge still succeeds. Any project-output residue outside
those two trees is left for `/spec:execute` to commit as
`specify: residue <slice-name>`. Committed changes remain local until the
operator explicitly runs `specify workspace push`.

## Summary template

Render after `specify slice merge run` succeeds, using `merged-specs[]` from
the response:

```text
Merge Complete

Slice:     <slice-name>
Merged to: .specify/archive/YYYY-MM-DD-<name>/

Specs Merged
- <adapter-1>: merged into .specify/specs/<adapter-1>/spec.md
- <adapter-2>: new baseline created at .specify/specs/<adapter-2>/spec.md

Decisions Promoted
- DEC-0007: Use PostgreSQL for the identity store
- DEC-0008: DPoP sender-constrained access tokens (supersedes DEC-0003)

(or "No delta specs to merge" if `specify slice merge preview` returned an
empty `specs` array)

All artifacts complete. All tasks complete.
```

Render the **Decisions Promoted** block from the `decisions[]` ids on the
`slice.archive.created` ledger entry the merge appended (the `DEC-NNNN` ids of
the records promoted into `.specify/decisions/`). Omit the block when the slice
authored no Decision Records — it stays off an empty merge.

Mention any workspace auto-commit warning or residue note returned by the CLI.

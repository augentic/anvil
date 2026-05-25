# Merge skill — runbook detail

The merge SKILL.md keeps the algorithmic spine; this file owns the verbatim
output templates and the workspace-clone commit semantics that step 5 invokes.

## Preview template

Render this from the `operations[]` array returned by
`specrun slice merge preview --format json`. Operations are typed `added`,
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

If `specrun slice merge preview` returns an empty `specs` array, report "No
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
`.specify/project.yaml`), `specrun slice merge run` auto-commits **only**
`.specify/specs/` and `.specify/archive/` with message
`specify: merge <slice-name>`. Commit failure is a **warning**, not an
error — the spec merge still succeeds. Any project-output residue outside
those two trees is left for `/spec:execute` to commit as
`specify: residue <slice-name>`. Committed changes remain local until the
operator explicitly runs `specrun workspace push`.

## Summary template

Render after `specrun slice merge run` succeeds, using `merged-specs[]` from
the response:

```text
Merge Complete

Slice:     <slice-name>
Merged to: .specify/archive/YYYY-MM-DD-<name>/

Specs Merged
- <adapter-1>: merged into .specify/specs/<adapter-1>/spec.md
- <adapter-2>: new baseline created at .specify/specs/<adapter-2>/spec.md

(or "No delta specs to merge" if `specrun slice merge preview` returned an
empty `specs` array)

All artifacts complete. All tasks complete.
```

Mention any workspace auto-commit warning or residue note returned by the CLI.

# Change skills

Change-scoped skills coordinate multi-slice changes through `change.md` + `plan.yaml`. They sit above the [slice lifecycle skills](../slice-skills/index.md) and invoke them per-slice. All change-scoped skills (`/change:analyze`, `/change:draft`, `/change:execute`, `/change:finalize`) live on the `change` plugin.

The change layer is split into three peer skills with an explicit human seam between authoring and execution. The lifecycle reads `draft → execute → finalize`, mirroring `/spec`'s `define → build → merge` rhythm at the change layer. There is no umbrella mode; the human pause between authoring and execution is the design.

## The three-skill lifecycle

```text
/change:draft <name>  →  operator review  →  /change:execute loop  →  /change:finalize <name>
        │                       │                     │                        │
        │                       │                     │                        │
        ▼                       ▼                     ▼                        ▼
   author plan.yaml,     specify plan amend,   per-slice define →        specify workspace push,
   stop at hand-off      specify plan status   build → merge until       gh pr list, specify
                                               no eligible slice         change finalize
                                               remains
```

`/change:draft` produces the plan and stops. The operator reviews `plan.yaml` (and may edit it via `specify plan amend`). `/change:execute` consumes the plan by running define-build-merge per slice in dependency order. `/change:finalize` pushes branches, observes PR state, and archives the change once every PR is `MERGED`.

Re-entry across all three skills: fix the cause, re-run the same skill. Nothing tracks "where the operator was" outside `plan.yaml`, `change.md`, and the on-disk brief artefacts.

## Skill summary

| Skill | Purpose | Reads | Writes |
|-------|---------|-------|--------|
| [/change:analyze](analyze.md) | Plan-time capability inference (invoked internally by `/change:draft`) | Source code or documentation | `discovery.md`, optional `metadata.json` |
| [/change:draft](draft.md) | Author `plan.yaml` from inputs; stop at the operator review seam | Sources, docs, registry, baseline specs | `plan.yaml`, `change.md`, `discovery.md`, `proposal.md`, optional `workspace.md`; for multi-project plans, amends entries with the CLI project option via the assignment step |
| [/change:execute](execute.md) | Drive the plan through define-build-merge per slice; supports supervised, `dry-run`, and `loop` modes with self-heal | `plan.yaml` | Plan status transitions (via CLI); prepares workspace branches, routes into workspace clones for multi-project plans, and commits non-baseline residue after merge |
| [/change:finalize](finalize.md) | Push branches, observe PR state, run `specify change finalize` once every PR is `MERGED` | `plan.yaml`, workspace clones, remote PR state | Composition only — shells out to `specify workspace push`, `gh pr list`, `specify change finalize`; never writes directly |

## Layered composition

These skills are optional. You can use the define-build-merge loop without ever touching plans. But when you do need them, they compose:

- **Plan authoring alone (`/change:draft`)** — author a plan, then drive it manually with the CLI.
- **Plan + drive (`/change:draft` then `/change:execute`)** — author a plan, then automate execution.
- **Full lifecycle (`/change:draft` then `/change:execute` then `/change:finalize`)** — author, drive, and close out a change end-to-end across three skills with an explicit operator review seam between draft and execute.
- **Single slice** — skip plans entirely, define and build slices one at a time.

The underlying CLI commands (`specify plan ...`, `specify workspace ...`, `specify change ...`) remain available as manual fallback at every level. There is no one-command convenience wrapper; teams that want one can compose the three skills in their own shell script, accepting that the wrapper opts out of the operator review pause.

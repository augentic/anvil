# Slice skills

Slice skills operate on a single slice inside `.specify/slices/<name>/`. This reference covers the deterministic breakouts that run outside the `/spec:execute` loop or land artifacts at the slice level. The full per-slice loop is `refining → built → merged` (see [Lifecycle](../lifecycle.md)).

## The per-slice loop

```text
/spec:init  →  (plan-time)  →  /spec:refine  →  /spec:build  →  /spec:merge
```

`/spec:init` is one-time scaffolding. The per-slice loop runs inside `/spec:execute`, but every step is also reachable as a breakout when execute parks or when the operator wants to drive a slice by hand. `/spec:refine` is documented in the operator guide (it is plan-resolved and consumes the active `in-progress` entry from `specify plan next`); this reference covers the breakouts an operator most often invokes directly.

## Skill summary

| Skill                             | Purpose                                       | Reads                                                | Writes                                              |
| --------------------------------- | --------------------------------------------- | ---------------------------------------------------- | --------------------------------------------------- |
| [/spec:init](init.md)              | One-time project setup                        | --                                                   | `.specify/`, `project.yaml`, cache, `AGENTS.md`     |
| [/spec:build](build.md)            | Validate artifacts, implement tasks            | Slice artifacts, target build brief                  | Source code, task checkmarks                        |
| [/spec:merge](merge.md)            | Apply slice deltas to baseline, archive slice  | Slice specs, baseline                                | Updated baseline, archived slice, per-entry `done`  |
| [/spec:drop](drop.md)              | Discard a slice without merging                | Slice metadata                                        | Archived slice (dropped)                            |

## How skills delegate

Each skill is an agent-driven orchestrator. Deterministic operations are delegated to the `specify` CLI. Skills never hand-edit `.metadata.yaml`, never create directories under `.specify/`, and never move files to the archive directly.

During `/spec:build`, tasks with skill directive tags (e.g. `<!-- skill: omnia:crate-writer -->`) are delegated to the named specialist plugin skill. Tasks without tags are implemented via the target adapter's default `build` brief.

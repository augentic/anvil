# Slice skills

Slice skills operate on a single slice inside `.specify/slices/<name>/`. They cover one-time project setup and the per-slice refine → build → merge loop. Change-level skills ([/spec:plan](../change-skills/plan.md), [/spec:execute](../change-skills/execute.md), [/spec:finalize](../change-skills/finalize.md)) sequence these skills inside `/spec:execute`; every step is also reachable as a manual breakout when execute parks.

## The per-slice loop

```text
/spec:init  →  (plan-time)  →  /spec:refine  →  /spec:build  →  /spec:merge
```

`/spec:init` is one-time scaffolding. The loop runs inside `/spec:execute`, but each phase is invokable by hand. See [Drive a slice manually](../../how-to/drive-slice-manually.md).

## Skill summary

| Skill | Purpose | Reads | Writes |
| ----- | ------- | ----- | ------ |
| [/spec:init](init.md) | One-time project setup | — | `.specify/`, `project.yaml`, cache, `AGENTS.md` |
| [/spec:refine](refine.md) | Extract per source, synthesize artifacts | Plan bindings, discovery, sources | Slice artifacts, Evidence, `provenance.yaml` |
| [/spec:build](build.md) | Validate artifacts, implement tasks | Slice artifacts, target build brief | Source code, task checkmarks |
| [/spec:merge](merge.md) | Apply slice deltas to baseline, archive slice | Slice specs, baseline | Updated baseline, archived slice, per-entry `done` |
| [/spec:drop](drop.md) | Discard a slice without merging | Slice metadata | Archived slice (dropped) |

## How skills delegate

Each skill is an agent-driven orchestrator. Deterministic operations are delegated to the `specify` CLI. Skills never hand-edit `.metadata.yaml`, never create directories under `.specify/` with shell tools, and never move files to the archive directly.

During `/spec:build`, tasks with skill directive tags (e.g. `<!-- skill: omnia:crate-writer -->`) delegate to named specialist plugin skills. Tasks without tags follow the target adapter's default `build` brief.

## See also

- [Change skills](../change-skills/index.md) — plan, execute, finalize
- [Lifecycle](../lifecycle.md) — slice and per-entry state machines
- [Quick reference card](../quick-reference.md) — all skills at a glance

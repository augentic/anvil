# Slice skills

Slice skills operate on a single slice inside `.specify/slices/<name>/`. They form the core define-build-merge loop and provide supporting adapters for extracting artifacts from existing source.

## The define-build-merge loop

```text
/spec:init  -->  /spec:define  -->  /spec:build  -->  /spec:merge
```

This is the primary workflow. You initialise a project once, then repeat the define-build-merge cycle for each slice.

## Skill summary

| Skill | Purpose | Reads | Writes |
|-------|---------|-------|--------|
| [/spec:init](init.md) | One-time project setup | -- | `.specify/`, `project.yaml`, cache, `AGENTS.md` |
| [/spec:define](define.md) | Generate all artifacts for a new change | Adapter briefs, baseline specs | `proposal.md`, `spec.md`, `composition.yaml`*, `design.md`, `tasks.md` |
| [/spec:build](build.md) | Implement tasks from a defined change | All artifacts, build brief | Source code, task checkmarks |
| [/spec:merge](merge.md) | Merge completed slice into baseline | Change specs + composition*, baseline | Updated baseline, archived change |
| [/spec:drop](drop.md) | Discard a slice without merging | Change metadata | Archived change (dropped) |
| [/spec:extract](extract.md) | Produce specs and design from existing code | Source code | `spec.md`, `design.md` |

*\* `composition.yaml` is Vectis-adapter only.*

## How skills delegate

Each skill is an agent-driven orchestrator. Deterministic operations are delegated to the `specify` CLI. Skills never hand-edit `.metadata.yaml`, never create directories under `.specify/`, and never move files to the archive directly.

During `/spec:build`, tasks with skill directive tags (e.g. `<!-- skill: omnia:crate-writer -->`) are delegated to the named specialist plugin skill. Tasks without tags are implemented via the adapter's default build instruction.

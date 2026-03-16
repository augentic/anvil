---
name: status
description: Show the current state of Specify changes -- active changes, artifact completion, and task progress. Use when the user wants to check where they are.
license: MIT
---

# Status

Show the current state of Specify in this project.

## Input

Optionally specify a change name to focus on. Otherwise show an overview.

## Steps

1. **Check initialization and resolve schema**

   Verify `.specify/config.yaml` exists. If not:
   > "Specify is not initialized in this project. Run `/spec:init` to get started."

   Read `.specify/config.yaml` for the `schema` value and **resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`. Read `schema.yaml` to get the blueprint definitions (id, generates, requires) and build configuration.

2. **List active changes**

   List directories in `.specify/changes/`, skipping `archive/`. For each directory that contains a `.metadata.yaml` file, it is an active change.

   If no active changes exist, report: "No active changes."

3. **For each active change (or the one specified), show lifecycle status and artifact completion**

   Read `.metadata.yaml` for the change to get `status`, `schema`, `created_at`, `defined_at`, `build_started_at`, and `completed_at`.

   Display the lifecycle status prominently:
   - `defining` — "Definition in progress (artifacts may be incomplete)"
   - `defined` — "All artifacts created, ready for implementation"
   - `building` — "Implementation in progress"
   - `complete` — "All tasks complete, ready to promote"
   - `dropped` — "Change discarded and moved to archive without merging specs"

   For each blueprint defined in `schema.yaml`, check whether it is complete:
   - If `generates` is a simple filename (e.g., `proposal.md`), check if `.specify/changes/<name>/<generates>` exists.
   - If `generates` is a glob pattern (e.g., `specs/**/*.md`), check if the directory contains at least one matching `.md` file.

   Derive readiness from each blueprint's `requires` field:
   - A blueprint with empty `requires` is always **ready** (no dependencies)
   - A blueprint is **ready** when all blueprints listed in its `requires` are complete
   - A blueprint is **blocked** when any blueprint in its `requires` is incomplete
   - A blueprint is **done** when its generated file(s) exist

   Display the blueprint table dynamically from the schema's blueprint list.

4. **Check task progress**

   If the artifact tracked by `build.tracks` (from `schema.yaml`) exists, read it and count lines matching:
   - `- [ ]` = incomplete task
   - `- [x]` or `- [X]` = complete task

   Report: "N/M tasks complete"

5. **Check build readiness**

   Build is ready when all blueprints listed in `build.requires` (from `schema.yaml`) are complete.

6. **Show next-step guidance based on lifecycle status**

   Based on the `status` field, provide targeted guidance:
   - `defining` — "Run `/spec:define` to complete artifact generation, or `/spec:drop` to discard the change."
   - `defined` — "Run `/spec:build` to start implementing tasks, or `/spec:drop` to discard the change."
   - `building` — "Run `/spec:build` to continue implementation, or `/spec:drop` to discard the change." Show remaining task count.
   - `complete` — "Run `/spec:promote` to finalize this change, or `/spec:drop` to discard it without merging specs."

7. **List archived changes** (brief)

   List directories in `.specify/changes/archive/` if any exist. If an archived directory contains `.metadata.yaml`, read its `status` and show whether it was `promoted` or `dropped`.

## Output

```text
## Specify Status

### Active Changes

**<change-name>** (schema: omnia, status: defined, created: <date>)

| Artifact | Status |
|----------|--------|
| proposal | done   |
| specs    | done   |
| design   | done   |
| tasks    | done   |

Tasks: 0/5 complete
Build: ready

Next: Run `/spec:build` to start implementing tasks.

### Archived Changes

- 2026-01-15-add-auth (status: promoted)
- 2026-02-01-spike-export (status: dropped)
```

If a single change is specified or only one exists, show the detailed view only (skip the list format).

## Guardrails

- Read-only -- do not create or modify any files
- If `.specify/` does not exist, suggest `/spec:init`
- Show clear next-step guidance based on current lifecycle status
- Distinguish promoted changes from dropped changes when metadata is available

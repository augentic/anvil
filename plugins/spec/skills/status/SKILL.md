---
name: status
description: Show the current state of Specify changes -- active changes, artifact completion, and task progress. Use when the user wants to check where they are.
license: MIT
argument-hint: "[change-name?]"
---

# Status

Show the current state of Specify in this project.

## Input

Optionally specify a change name to focus on. Otherwise show an overview.

## Steps

1. **Check initialization and resolve schema**

   Verify `.specify/project.yaml` exists. If not:
   > "Specify is not initialized in this project. Run `/spec:init` to get started."

   Read `.specify/project.yaml` for the `schema` value and **resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`. Read `schema.yaml` to get the pipeline definitions. For each entry in `pipeline.define`, `pipeline.build`, and `pipeline.merge`, read the brief's frontmatter for `id`, `generates`, `needs`, and `tracks`.

2. **List active changes**

   List directories in `.specify/changes/`, skipping `archive/`. For each directory that contains a `.metadata.yaml` file, it is an active change.

   If no active changes exist, report: "No active changes."

3. **For each active change (or the one specified), show lifecycle status and artifact completion**

   Read `.metadata.yaml` for the change to get `status`, `schema`, `created_at`, `defined_at`, `build_started_at`, and `completed_at`.

   Display the lifecycle status prominently:
   - `defining` — "Definition in progress (artifacts may be incomplete)"
   - `defined` — "All artifacts created, ready for implementation"
   - `building` — "Implementation in progress"
   - `complete` — "All tasks complete, ready to merge"
   - `dropped` — "Change discarded and moved to archive without merging specs"

   For each brief in `pipeline.define` entries, check whether it is complete:
   - If `generates` is a simple filename (e.g., `proposal.md`), check if `.specify/changes/<name>/<generates>` exists.
   - If `generates` is a glob pattern (e.g., `specs/**/*.md`), check if the directory contains at least one matching `.md` file.

   Derive readiness from each brief's `needs` field:
   - A brief with empty `needs` is always **ready** (no dependencies)
   - A brief is **ready** when all briefs listed in its `needs` are complete
   - A brief is **blocked** when any brief in its `needs` is incomplete
   - A brief is **done** when its generated file(s) exist

   Display the artifact table dynamically from the pipeline's brief list.

4. **Check task progress**

   Read the build brief (from `pipeline.build`) and check its frontmatter `tracks` field. If the tracked artifact exists, read it and count lines matching:
   - `- [ ]` = incomplete task
   - `- [x]` or `- [X]` = complete task

   Report: "N/M tasks complete"

5. **Check build readiness**

   Build is ready when all briefs listed in the build brief's `needs` are complete.

6. **Show next-step guidance based on lifecycle status**

   Based on the `status` field, provide targeted guidance:
   - `defining` — "Run `/spec:define` to complete artifact generation, or `/spec:drop` to discard the change."
   - `defined` — "Run `/spec:build` to start implementing tasks, or `/spec:drop` to discard the change."
   - `building` — "Run `/spec:build` to continue implementation, or `/spec:drop` to discard the change." Show remaining task count.
   - `complete` — "Run `/spec:merge` to finalize this change, or `/spec:drop` to discard it without merging specs. Consider running `/spec:verify` before merging to confirm code matches specs."

7. **List archived changes** (brief)

   List directories in `.specify/changes/archive/` if any exist. If an archived directory contains `.metadata.yaml`, read its `status` and show whether it was `merged` or `dropped`.

   If baseline specs exist at `.specify/specs/`, note: "Use `/spec:verify` at any time to detect drift between code and baseline specs."

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

- 2026-01-15-add-auth (status: merged)
- 2026-02-01-spike-export (status: dropped)
```

If a single change is specified or only one exists, show the detailed view only (skip the list format).

## Guardrails

- Read-only -- do not create or modify any files
- If `.specify/` does not exist, suggest `/spec:init`
- Show clear next-step guidance based on current lifecycle status
- Distinguish merged changes from dropped changes when metadata is available

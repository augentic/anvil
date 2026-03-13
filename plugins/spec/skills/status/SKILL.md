---
name: status
description: Show the current state of Specify changes -- active changes, artifact completion, and task progress. Use when the user wants to check where they are.
license: MIT
metadata:
  author: specify
  version: "3.0"
---

# Status

Show the current state of Specify in this project.

## Input

Optionally specify a change name to focus on. Otherwise show an overview.

## Steps

1. **Check initialization and resolve schema**

   Verify `.specify/config.yaml` exists. If not:
   > "Specify is not initialized in this project. Run `/spec:init` to get started."

   Read `.specify/config.yaml` for the `schema` value and **resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`. Read `schema.yaml` to get the artifact definitions (id, generates, requires) and apply configuration.

2. **List active changes**

   List directories in `.specify/changes/`, skipping `archive/`. For each directory that contains a `.metadata.yaml` file, it is an active change.

   If no active changes exist, report: "No active changes."

3. **For each active change (or the one specified), show lifecycle status and artifact completion**

   Read `.metadata.yaml` for the change to get `status`, `schema`, `created_at`, `proposed_at`, `apply_started_at`, and `completed_at`.

   Display the lifecycle status prominently:
   - `proposing` — "Proposal in progress (artifacts may be incomplete)"
   - `proposed` — "All artifacts created, ready for review or implementation"
   - `reviewed` — "All structured checks passed, ready for implementation"
   - `applying` — "Implementation in progress"
   - `complete` — "All tasks complete, ready to archive"
   - `abandoned` — "Change discarded and moved to archive without merging specs"

   For each artifact defined in `schema.yaml`, check whether it is complete:
   - If `generates` is a simple filename (e.g., `proposal.md`), check if `.specify/changes/<name>/<generates>` exists.
   - If `generates` is a glob pattern (e.g., `specs/**/*.md`), check if the directory contains at least one matching `.md` file.

   Derive readiness from each artifact's `requires` field:
   - An artifact with empty `requires` is always **ready** (no dependencies)
   - An artifact is **ready** when all artifacts listed in its `requires` are complete
   - An artifact is **blocked** when any artifact in its `requires` is incomplete
   - An artifact is **done** when its generated file(s) exist

   Display the artifact table dynamically from the schema's artifact list.

4. **Check task progress**

   If the artifact tracked by `apply.tracks` (from `schema.yaml`) exists, read it and count lines matching:
   - `- [ ]` = incomplete task
   - `- [x]` or `- [X]` = complete task

   Report: "N/M tasks complete"

5. **Check apply readiness**

   Apply is ready when all artifacts listed in `apply.requires` (from `schema.yaml`) are complete.

6. **Show next-step guidance based on lifecycle status**

   Based on the `status` field, provide targeted guidance:
   - `proposing` — "Run `/spec:propose` to complete artifact generation, or `/spec:abandon` to discard the change."
   - `proposed` — "Run `/spec:review` to validate artifacts, `/spec:apply` to start implementing tasks, or `/spec:abandon` to discard the change."
   - `reviewed` — "Run `/spec:apply` to start implementing tasks, or `/spec:abandon` to discard the change."
   - `applying` — "Run `/spec:apply` to continue implementation, or `/spec:abandon` to discard the change." Show remaining task count.
   - `complete` — "Run `/spec:archive` to finalize this change, or `/spec:abandon` to discard it without merging specs."

7. **List archived changes** (brief)

   List directories in `.specify/changes/archive/` if any exist. If an archived directory contains `.metadata.yaml`, read its `status` and show whether it was `archived` or `abandoned`.

## Output

```text
## Specify Status

### Active Changes

**<change-name>** (schema: omnia, status: proposed, created: <date>)

| Artifact | Status |
|----------|--------|
| proposal | done   |
| specs    | done   |
| design   | done   |
| tasks    | done   |

Tasks: 0/5 complete
Apply: ready

Next: Run `/spec:apply` to start implementing tasks.

### Archived Changes

- 2026-01-15-add-auth (status: archived)
- 2026-02-01-spike-export (status: abandoned)
```

If a single change is specified or only one exists, show the detailed view only (skip the list format).

## Guardrails

- Read-only -- do not create or modify any files
- If `.specify/` does not exist, suggest `/spec:init`
- Show clear next-step guidance based on current lifecycle status
- Distinguish archived changes from abandoned changes when metadata is available

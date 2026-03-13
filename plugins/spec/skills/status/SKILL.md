---
name: status
description: Show the current state of Specify changes -- active changes, artifact completion, and task progress. Use when the user wants to check where they are.
license: MIT
metadata:
  author: specify
  version: "2.0"
---

Show the current state of Specify in this project.

---

**Input**: Optionally specify a change name to focus on. Otherwise show an overview.

**Steps**

1. **Check initialization and resolve schema**

   Verify `.specify/config.yaml` exists. If not:
   > "Specify is not initialized in this project. Run `/spec:init` to get started."

   Read `.specify/config.yaml` for the `schema` value and **resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`. Read `schema.yaml` to get the artifact definitions (id, generates, requires) and apply configuration.

2. **List active changes**

   List directories in `.specify/changes/`, skipping `archive/`. For each directory that contains a `.metadata.yaml` file, it is an active change.

   If no active changes exist, report: "No active changes."

3. **For each active change (or the one specified), check artifact completion**

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
   - `- [ ] ` = incomplete task
   - `- [x] ` or `- [X] ` = complete task

   Report: "N/M tasks complete"

5. **Check apply readiness**

   Apply is ready when all artifacts listed in `apply.requires` (from `schema.yaml`) are complete.

6. **List archived changes** (brief)

   List directories in `.specify/changes/archive/` if any exist.

**Output**

```
## Specify Status

### Active Changes

**<change-name>** (schema: omnia, created: <date>)

| Artifact | Status |
|----------|--------|
| proposal | done   |
| specs    | done   |
| design   | done   |
| tasks    | ready  |

Tasks: 0/5 complete
Apply: blocked (tasks not complete)

### Archived Changes

- 2026-01-15-add-auth
- 2026-02-01-fix-export
```

If a single change is specified or only one exists, show the detailed view only (skip the list format).

**Guardrails**
- Read-only -- do not create or modify any files
- If `.specify/` does not exist, suggest `/spec:init`
- Show clear next-step guidance based on current state

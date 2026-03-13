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

1. **Check initialization**

   Verify `.specify/config.yaml` exists. If not:
   > "Specify is not initialized in this project. Run `/spec:init` to get started."

2. **List active changes**

   List directories in `.specify/changes/`, skipping `archive/`. For each directory that contains a `.metadata.yaml` file, it is an active change.

   If no active changes exist, report: "No active changes."

3. **For each active change (or the one specified), check artifact completion**

   Check file existence to determine artifact status:

   | Artifact | Complete when |
   |----------|---------------|
   | proposal | `.specify/changes/<name>/proposal.md` exists |
   | specs | `.specify/changes/<name>/specs/` contains at least one `.md` file (in any subdirectory) |
   | design | `.specify/changes/<name>/design.md` exists |
   | tasks | `.specify/changes/<name>/tasks.md` exists |

   Dependency order for readiness:
   - `proposal`: always ready (no dependencies)
   - `specs`: ready when `proposal` is complete
   - `design`: ready when `proposal` is complete
   - `tasks`: ready when both `specs` and `design` are complete

   Classify each artifact as:
   - **done**: file exists
   - **ready**: dependencies are complete, file is missing
   - **blocked**: dependencies are incomplete

4. **Check task progress** (if tasks.md exists)

   Read `tasks.md` and count lines matching:
   - `- [ ] ` = incomplete task
   - `- [x] ` or `- [X] ` = complete task

   Report: "N/M tasks complete"

5. **Check apply readiness**

   Apply is ready when `tasks.md` exists (the `tasks` artifact is complete).

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

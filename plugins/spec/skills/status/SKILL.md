---
name: status
description: Show the current state of Specify changes. Invokes `specify status` and renders active changes, artifact completion, and task progress. Use when the user wants to check where they are.
license: MIT
argument-hint: "[change-name?]"
---

# Status

## Prerequisites

**If `specify` is not on PATH:** stop and instruct the user to install the CLI via `brew install augentic/tap/specify` (preferred), `cargo install specify`, or the release script at https://specify.sh/install, then re-run. Do not attempt a prose fallback — validation rules have diverged past the point where the agent can reliably reproduce them.

Show the current state of Specify in this project.

## Input

Optionally specify a change name to focus on. Otherwise show an overview.

## Steps

1. **Check initialization**

   Verify `.specify/project.yaml` exists. If not:

   > "Specify is not initialized in this project. Run `/spec:init` to get started."

2. **Gather status from the CLI**

   ```bash
   specify status ${CHANGE:+"$CHANGE"} --format json
   ```

   The response contains a `changes` array; each entry has `name`, `status`, `schema`, `tasks` (`{ total, complete }` or `null`), and `artifacts` (map of brief id → `bool`). On non-zero exit surface the JSON `error`/`message` and stop. If `changes` is empty, report "No active changes."

3. **Render the output**

   For each entry, display the lifecycle `status`, the `schema`, per-brief artifact completion, and task progress. Pick a lifecycle blurb:

   - `defining` — "Definition in progress (artifacts may be incomplete)"
   - `defined` — "All artifacts created, ready for implementation"
   - `building` — "Implementation in progress"
   - `complete` — "All tasks complete, ready to merge"
   - `merged` — "Merged into baseline"
   - `dropped` — "Change discarded and moved to archive without merging specs"

4. **Next-step guidance**

   Based on `status`, suggest one of:

   - `defining` — "Run `/spec:define` to complete artifact generation, or `/spec:drop` to discard the change."
   - `defined` — "Run `/spec:build` to start implementing tasks, or `/spec:drop` to discard."
   - `building` — "Run `/spec:build` to continue implementation, or `/spec:drop` to discard." Show remaining task count.
   - `complete` — "Run `/spec:merge` to finalize, or `/spec:drop` to discard. Consider `/spec:verify` before merging."

5. **List archived changes** (when showing the overview, not a single change)

   List directories in `.specify/archive/` if any exist. If an archived directory contains `.metadata.yaml`, read its `status` and show whether it was `merged` or `dropped`.

   If baseline specs exist at `.specify/specs/`, note: "Use `/spec:verify` at any time to detect drift between code and baseline specs."

## Output

```text
## Specify Status

### Active Changes

**<change-name>** (schema: omnia, status: defined)

| Artifact | Status |
|----------|--------|
| proposal | done   |
| specs    | done   |
| design   | done   |
| tasks    | done   |

Tasks: 0/5 complete

Next: Run `/spec:build` to start implementing tasks.

### Archived Changes

- 2026-01-15-add-auth (status: merged)
- 2026-02-01-spike-export (status: dropped)
```

If a single change is specified or only one exists, show the detailed view only (skip the list format).

## Guardrails

- Read-only — do not create or modify any files
- If `.specify/` does not exist, suggest `/spec:init`
- Show clear next-step guidance based on current lifecycle status
- Distinguish merged changes from dropped changes when metadata is available

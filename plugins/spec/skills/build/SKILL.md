---
name: build
description: Implement tasks from a Specify change. Invokes `specify validate`, `specify task progress`, and `specify task mark`; agent owns the implementation loop. Use when the user wants to start implementing, continue implementation, or work through tasks.
license: MIT
argument-hint: "[change-name?]"
---

## Prerequisites

**If `specify` is not on PATH:** stop and instruct the user to install the
CLI via `brew install specify` (preferred), `cargo install specify`, or
the release script at https://specify.sh/install, then re-run. Do not
attempt a prose fallback — validation rules have diverged past the point
where the agent can reliably reproduce them.

Implement tasks from a Specify change.

**Input**: Optionally specify a change name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

**Steps**

1. **Select the change**

   If a name is provided, use it. Otherwise:
   - Infer from conversation context if the user mentioned a change
   - Auto-select if only one active change exists (list directories in `.specify/changes/`, skipping `archive/`, looking for dirs with `.metadata.yaml`)
   - If ambiguous, list available changes and use the **AskQuestion tool** to let the user select

   Always announce: "Using change: <name>" and how to override (e.g., `/spec:build <other>`).

   Set `$CHANGE_DIR = .specify/changes/<name>`.

2. **Read project config and resolve schema**

   Read `.specify/project.yaml` for project domain and rule overrides.

   Read `$CHANGE_DIR/.metadata.yaml` for the schema value and status.

   **Resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`, `briefs/build.md`.

   **Resolve effective domain**: use the project's `domain` (from `.specify/project.yaml`) if present and non-empty, otherwise fall back to the schema's `domain`. **Resolve effective rules**: for each brief ID under `rules`, use the project's value if present and non-empty. Use effective domain and effective rules as constraints guiding your implementation — do not copy them into code comments.

3. **Check lifecycle status**

   Read `status` from `.metadata.yaml`:
   - If `status` is `defining`: warn that artifacts may be incomplete. Suggest `/spec:define` to complete them.
   - If `status` is `complete`: congratulate, all tasks already done. Suggest `/spec:merge`.
   - Otherwise: proceed.

4. **Read context files**

   Read all artifacts for the change. For each brief in `pipeline.define`, read the file(s) using the brief's frontmatter `generates` path. For glob patterns (e.g. `specs/**/*.md`), read all matching files.

5. **Validate needs**

   Validate that every brief listed in the build brief's `needs` frontmatter has its generated artifact present and non-empty in the change directory. If any needed artifact is missing, halt and report which artifacts are missing — suggest `/spec:define` to create them.

6. **Validate artifacts**

   ```bash
   specify validate "$CHANGE_DIR" --format json
   ```

   If `passed` is false: report failures to the user and suggest fixes.
   If any results have `status: deferred`: apply your judgment for those rules.
   Do not proceed to implementation until all non-deferred checks pass.

7. **Show current progress**

   ```bash
   specify task progress "$CHANGE_DIR" --format json
   ```

   Display `complete/total` tasks and the remaining-tasks list from the `tasks` array. If `complete == total`: congratulate and suggest `/spec:merge`.

8. **Update lifecycle status**

   If `status` in `.metadata.yaml` is `defined` (first time building):
   - Update `status` to `building`
   - Set `build_started_at` to current ISO-8601 timestamp
   - **Verify**: re-read `.metadata.yaml` and confirm the `status` value is exactly `building`. Valid lifecycle values are: `defining`, `defined`, `building`, `complete`, `merged`, `dropped`.

9. **Implement tasks (loop until done or blocked)**

   Read the build brief from the resolved schema directory.

   **Skill directive tags**: Before starting each task, check whether it contains an HTML comment in the form `<!-- skill: plugin:skill-name -->`. If present, invoke that skill directly instead of following default mode-detection logic. Tasks without a skill tag follow the instruction file's mode detection and step-by-step execution.

   For each pending task (discovered via `specify task progress`):
   - Check for a skill directive tag and invoke the named skill if present
   - Otherwise follow the instruction file (arguments, mode detection, step-by-step execution)
   - Show which task is being worked on
   - Make the code changes required — keep changes minimal and focused
   - Mark the task complete via the CLI:

     ```bash
     specify task mark "$CHANGE_DIR" "$TASK_NUMBER" --format json
     ```

     The response is idempotent (`idempotent: true` means the task was already complete — keep going).
   - Continue to next task.

   **Pause if:**
   - Task is unclear — ask for clarification
   - Implementation reveals a design issue — suggest updating artifacts (`/spec:define <name> <artifact-id>` to regenerate)
   - Error or blocker encountered — report and wait for guidance
   - User interrupts

10. **On completion or pause, show status**

    If all tasks are complete:
    - Update `.metadata.yaml`: set `status` to `complete`, set `completed_at` to current ISO-8601 timestamp
    - **Verify**: re-read `.metadata.yaml` and confirm the `status` value is exactly `complete`. Valid lifecycle values are: `defining`, `defined`, `building`, `complete`, `merged`, `dropped`.

    Display:
    - Tasks completed this session
    - Overall progress: "N/M tasks complete" (via `specify task progress`)
    - If all done: suggest `/spec:merge`
    - If paused: explain why and wait for guidance

**Output During Implementation**

```
## Implementing: <change-name>

Working on task 3/7: <task description>
[...implementation happening...]
Task complete

Working on task 4/7: <task description>
[...implementation happening...]
Task complete
```

**Output On Completion**

```
## Implementation Complete

**Change:** <change-name>
**Progress:** 7/7 tasks complete

### Completed This Session
- [x] Task 1
- [x] Task 2
...

All tasks complete! Ready to merge this change.
Run `/spec:merge` to finalize.
```

**Output On Pause (Issue Encountered)**

```
## Implementation Paused

**Change:** <change-name>
**Progress:** 4/7 tasks complete

### Issue Encountered
<description of the issue>

**Options:**
1. <option 1>
2. <option 2>
3. Other approach

What would you like to do?
```

**Guardrails**
- **All artifacts live under `.specify/changes/<name>/`**. Read and write artifacts relative to this directory only. Do NOT use `openspec/`, `temp/`, or any other directory convention from other plugins or skills.
- Keep going through tasks until done or blocked
- Always read context files before starting
- If task is ambiguous, pause and ask before implementing
- If implementation reveals issues, pause and suggest artifact updates
- Keep code changes minimal and scoped to each task
- Use `specify task mark` to update checkboxes immediately after completing each task
- Pause on errors, blockers, or unclear requirements — don't guess
- Valid lifecycle status values are: `defining`, `defined`, `building`, `complete`, `merged`, `dropped` — use these exact strings when updating `.metadata.yaml`, no other values are permitted

**Fluid Workflow Integration**

This skill is not phase-locked: it can be invoked before all artifacts are done, after partial implementation, or interleaved with other actions. If implementation reveals design issues, suggest updating artifacts and continue.

> Implements RFC-1 Phase 1 — the CLI handles deterministic operations.

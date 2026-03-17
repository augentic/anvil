---
name: build
description: Implement tasks from a Specify change. Use when the user wants to start implementing, continue implementation, or work through tasks.
license: MIT
argument-hint: "[change-name?]"
---

Implement tasks from a Specify change.

**Input**: Optionally specify a change name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

**Steps**

1. **Select the change**

   If a name is provided, use it. Otherwise:
   - Infer from conversation context if the user mentioned a change
   - Auto-select if only one active change exists (list directories in `.specify/changes/`, skipping `archive/`, looking for dirs with `.metadata.yaml`)
   - If ambiguous, list available changes and use the **AskQuestion tool** to let the user select

   Always announce: "Using change: <name>" and how to override (e.g., `/spec:build <other>`).

2. **Read project config and resolve schema**

   Read `.specify/config.yaml` for project context overrides and rule overrides.

   Read `.specify/changes/<name>/.metadata.yaml` for the schema value and status.

   **Resolve the schema** using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`, `instructions/build.md`. Read `schema.yaml` from the resolved location.

   **Resolve effective context**: use the project's `context` (from `.specify/config.yaml`) if present and non-empty, otherwise fall back to the schema's `defaults.context` (from the resolved `schema.yaml`). **Resolve effective rules**: for each blueprint ID under `overrides`, use the project's value (from `.specify/config.yaml`) if present and non-empty, otherwise fall back to the schema's `defaults.rules` value (from the resolved `schema.yaml`). Use effective context and effective rules as constraints guiding your implementation -- do not copy them into code comments.

3. **Check lifecycle status**

   Read `status` from `.metadata.yaml`:
   - If `status` is `defining`: warn that artifacts may be incomplete — some may not have been generated yet. Suggest running `/spec:define` to complete them.
   - If `status` is `complete`: congratulate, all tasks already done. Suggest `/spec:merge`.
   - Otherwise: proceed.

4. **Check blueprint completion**

   For each blueprint defined in `schema.yaml`, check whether it is complete:
   - If `generates` is a simple filename (e.g., `proposal.md`), check if `.specify/changes/<name>/<generates>` exists.
   - If `generates` is a glob pattern (e.g., `specs/**/*.md`), check if the directory contains at least one matching `.md` file.

   **Handle states:**
   - If any blueprint listed in `build.requires` (from `schema.yaml`) is incomplete: show message listing missing artifacts, suggest using `/spec:define` to create them
   - Otherwise: proceed to implementation

5. **Read context files**

   Read all artifacts for the change. For each blueprint defined in `schema.yaml`, read the file(s) at `.specify/changes/<name>/<generates>`. For glob patterns (e.g., `specs/**/*.md`), read all matching files in the directory.

6. **Validate artifacts**

   Run all validation checks before proceeding to implementation. Collect all results — do not stop at the first failure.

   **Per-blueprint validation**: For each blueprint that has a `validate` field in `schema.yaml`, verify each rule against the artifact content read in step 5. Record each rule result as **PASS** or **FAIL** with a reason.

   **Cross-blueprint consistency checks**: For each key in `validation` (from `schema.yaml`) that is set to `true`, run the named check:
   - `proposal-crates-have-specs`: every crate listed in the proposal has a corresponding spec file under `specs/`
   - `design-references-valid`: requirement IDs (`REQ-XXX`) referenced in `design.md` exist in spec files
   - `spec-format-valid`: all spec files match the heading structure defined in `references/spec-format.md`

   Record each check result as **PASS** or **FAIL** with details.

   **If all checks pass**: report "Validation passed" and continue to step 7.

   **If any check fails**: produce a validation summary and **halt** — do not proceed to implementation. The example below uses the omnia schema's validation keys; the actual keys come from the `validation` section of the resolved `schema.yaml`.

   ```text
   ## Validation Failed: <change-name>

   ### Per-Blueprint Validation

   **proposal.md**
   - PASS: Has a Why section with at least one sentence
   - FAIL: Has a Crates section listing at least one new or modified crate — heading found but no content below it

   **specs/user-auth/spec.md**
   - PASS: Every requirement has at least one scenario
   - FAIL: Uses SHALL/MUST language for normative requirements — REQ-003 uses "should"

   **design.md**
   - PASS: Has a Context section

   **tasks.md**
   - PASS: Every task uses checkbox format

   ### Cross-Blueprint Checks

   - PASS: proposal-crates-have-specs
   - FAIL: design-references-valid — REQ-005 referenced in design.md not found in specs
   - PASS: spec-format-valid

   ### Result

   X passed, Y failed — fix the failures above before implementation can proceed.
   ```

   Suggest fixes for each failure:
   - Missing artifacts: "Run `/spec:define <name> <artifact-id>` to regenerate."
   - Spec format issues: "Edit the spec file to match the required structure."
   - Cross-artifact issues: "Update the referenced artifact to fix the inconsistency."

   Use heading conventions from `references/spec-format.md`. If `validate` is not defined for an artifact, skip validation for that artifact.

7. **Show current progress**

   Read the file tracked by `build.tracks` (from `schema.yaml`) and count:
   - `- [ ] ` lines = incomplete tasks
   - `- [x] ` or `- [X] ` lines = complete tasks

   Display:
   - Progress: "N/M tasks complete"
   - Remaining tasks overview

   If all tasks are already complete: congratulate, suggest `/spec:merge`.

8. **Update lifecycle status**

   If `status` in `.metadata.yaml` is `defined` (first time building):
   - Update `status` to `building`
   - Set `build_started_at` to current ISO-8601 timestamp

9. **Implement tasks (loop until done or blocked)**

   Read the build instruction file from the resolved schema directory (the file path is given by `build.instructions` in `schema.yaml`).

   **Skill directive tags**: Before starting each task, check whether it contains an HTML comment tag in the form `<!-- skill: plugin:skill-name -->`. If present, invoke that skill directly instead of following the default mode-detection logic. For example, a task tagged `<!-- skill: omnia:crate-writer -->` should be handled by running `/omnia:crate-writer` with the standard arguments. Tasks without a skill tag follow the instruction file's mode detection and step-by-step execution as before.

   For each pending task:
   - Check for a skill directive tag and invoke the named skill if present
   - Otherwise follow the instruction file (arguments, mode detection, step-by-step execution)
   - Show which task is being worked on
   - Make the code changes required
   - Keep changes minimal and focused
   - Mark task complete in the tasks file: `- [ ]` -> `- [x]`
   - Continue to next task

   **Pause if:**
   - Task is unclear -> ask for clarification
   - Implementation reveals a design issue -> suggest updating artifacts (use `/spec:define <name> <artifact-id>` to regenerate)
   - Error or blocker encountered -> report and wait for guidance
   - User interrupts

10. **On completion or pause, show status**

   If all tasks are complete:
   - Update `.metadata.yaml`: set `status` to `complete`, set `completed_at` to current ISO-8601 timestamp

   Display:
   - Tasks completed this session
   - Overall progress: "N/M tasks complete"
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
- Keep going through tasks until done or blocked
- Always read context files before starting
- If task is ambiguous, pause and ask before implementing
- If implementation reveals issues, pause and suggest artifact updates
- Keep code changes minimal and scoped to each task
- Update task checkbox immediately after completing each task
- Pause on errors, blockers, or unclear requirements -- don't guess

**Fluid Workflow Integration**

This skill supports the "actions on a change" model:

- **Can be invoked anytime**: Before all artifacts are done (if tasks exist), after partial implementation, interleaved with other actions
- **Allows artifact updates**: If implementation reveals design issues, suggest updating artifacts -- not phase-locked, work fluidly

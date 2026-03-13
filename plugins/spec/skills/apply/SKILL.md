---
name: apply
description: Implement tasks from a Specify change. Use when the user wants to start implementing, continue implementation, or work through tasks.
license: MIT
metadata:
  author: specify
  version: "2.0"
---

Implement tasks from a Specify change.

**Input**: Optionally specify a change name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

**Steps**

1. **Select the change**

   If a name is provided, use it. Otherwise:
   - Infer from conversation context if the user mentioned a change
   - Auto-select if only one active change exists (list directories in `.specify/changes/`, skipping `archive/`, looking for dirs with `.metadata.yaml`)
   - If ambiguous, list available changes and use the **AskQuestion tool** to let the user select

   Always announce: "Using change: <name>" and how to override (e.g., `/spec:apply <other>`).

2. **Check artifact completion**

   Verify all required artifacts exist by checking file presence:

   | Artifact | Complete when |
   |----------|---------------|
   | proposal | `.specify/changes/<name>/proposal.md` exists |
   | specs | `.specify/changes/<name>/specs/` contains at least one `.md` file (in any subdirectory) |
   | design | `.specify/changes/<name>/design.md` exists |
   | tasks | `.specify/changes/<name>/tasks.md` exists |

   **Handle states:**
   - If `tasks.md` does not exist (apply is blocked): show message listing missing artifacts, suggest using `/spec:propose` to create them
   - If all tasks are already complete: congratulate, suggest `/spec:archive`
   - Otherwise: proceed to implementation

3. **Read project config**

   Read `.specify/config.yaml` for project context. Use `context` and `rules` as constraints guiding your implementation -- do not copy them into code comments.

4. **Read context files**

   Read the artifacts for the change:
   - `.specify/changes/<name>/proposal.md`
   - `.specify/changes/<name>/specs/` (all spec files)
   - `.specify/changes/<name>/design.md`
   - `.specify/changes/<name>/tasks.md`

5. **Show current progress**

   Count tasks in `tasks.md`:
   - `- [ ] ` lines = incomplete tasks
   - `- [x] ` or `- [X] ` lines = complete tasks

   Display:
   - Progress: "N/M tasks complete"
   - Remaining tasks overview

6. **Implement tasks (loop until done or blocked)**

   Apply instruction (from schema):

   Arguments (used by all skills):
   - CHANGE_ID: the name of this change
   - PROJECT_DIR: the project root (typically ".")
   - CRATE_NAME: the spec folder name (`specs/<crate>/spec.md`)
   - CRATE_PATH: `PROJECT_DIR/crates/CRATE_NAME`

   Mode detection -- check whether `CRATE_PATH/Cargo.toml` exists:
   - If `Cargo.toml` does not exist, use **create mode**.
   - If `Cargo.toml` exists, use **update mode**.

   **Create mode** (Cargo.toml does NOT exist -- new crate):
   1. Use the `/omnia:guest-writer` skill to generate the WASM guest project (src/lib.rs, Cargo.toml, Provider). Run the skill from start to finish; do not implement the guest manually. Complete every step in the skill, and ensure the skill's verification checklist is satisfied.
   2. Use the `/omnia:crate-writer` skill to generate the domain crate (types, handlers, baseline tests) from Specify artifacts. Run the skill from start to finish; do not implement the crate manually. Complete every step in the skill, and ensure the skill's verification checklist is satisfied.
   3. Use the `/omnia:test-writer` skill to generate comprehensive test suites from spec scenarios. Run the skill from start to finish; do not implement the tests manually. Complete every step in the skill, and ensure the skill's verification checklist is satisfied.
   4. Verify from CRATE_PATH (repair loop -- max 3 iterations):
      a. `cargo fmt --check` -- If fails: run `cargo fmt`, then re-check.
      b. `cargo clippy -- -D warnings` -- If fails: fix each warning, then re-run.
      c. `cargo test` -- If fails: analyze each failure and apply the matching repair strategy (type mismatches, validation errors, provider mock errors, logic errors). Then re-run. Repeat from (a) until all three pass or 3 iterations are exhausted. If still failing: STOP. Do NOT mark the task complete. Report failures and ask for guidance.
   5. (Optional) `/omnia:code-reviewer <CRATE_PATH>` -- AI code review for security, WASM constraints, and quality.

   **Update mode** (Cargo.toml exists -- incremental change):
   1. Use the `/omnia:crate-writer` skill to update the domain crate. Run the skill from start to finish.
   2. Use the `/omnia:test-writer` skill to regenerate or update tests to match changed specs. Run the skill from start to finish.
   3. Verify from CRATE_PATH (repair loop -- max 3 iterations): same as create mode steps a/b/c.

   For each pending task:
   - Show which task is being worked on
   - Make the code changes required
   - Keep changes minimal and focused
   - Mark task complete in the tasks file: `- [ ]` -> `- [x]`
   - Continue to next task

   **Pause if:**
   - Task is unclear -> ask for clarification
   - Implementation reveals a design issue -> suggest updating artifacts
   - Error or blocker encountered -> report and wait for guidance
   - User interrupts

7. **On completion or pause, show status**

   Display:
   - Tasks completed this session
   - Overall progress: "N/M tasks complete"
   - If all done: suggest `/spec:archive`
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

All tasks complete! Ready to archive this change.
Run `/spec:archive` to finalize.
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

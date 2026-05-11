---
name: specify-build
description: Implement tasks from a Specify slice. Use when the user wants to start implementing, continue implementation, or work through tasks.
argument-hint: "[slice-name]"
---

## Critical Path (Quick Reference)

1. **Select the slice** — infer or ask for the active slice, then use `specify slice status <name> --format json` to confirm lifecycle.
2. **Preflight tasks and artifacts** — read `tasks.md` for agent-completability before running `specify slice validate <name> --format json`.
3. **Load build context** — resolve the build brief with `specify capability pipeline build`, then read tracked tasks and required define artifacts.
4. **Enter build state** — show task progress, transition `defined → building` through the CLI, and never edit `.metadata.yaml` directly.
5. **Implement pending tasks** — dispatch any `skill-directive` tasks to the named specialist; otherwise follow the build brief with focused code changes.
6. **Mark progress through CLI** — after each completed task, run `specify slice task mark <name> <task-number> --format json`; pause on ambiguity or blockers.
7. **Close or report outcome** — transition to `complete` when pending reaches zero, or apply the shared [phase outcome contract](../../references/phase-outcome-contract.md) for failure/deferred pauses.

Implement tasks from a Specify slice.

Deterministic bookkeeping — slice selection, lifecycle transitions, capability resolution, brief completion checks, task progress counting, checkbox flips — is delegated to the `specify` CLI. This skill drives the agent-side work: reading the build brief body, dispatching skill directives, and making code changes.

When working plan-driven (a `plan.yaml` exists), the corresponding entry should already be `in-progress`. `/spec:build` does not touch `plan.yaml`; plan-status transitions remain with the driver or human loop named in the shared [phase outcome contract](../../references/phase-outcome-contract.md).

## Phase outcome contract

This skill is the **build** phase of the `/change:execute` driver loop. Apply the shared [phase outcome contract](../../references/phase-outcome-contract.md), including build's per-phase deltas, journal rules, plan-mutation allowlist, and verbatim-`summary` rule.

**Input**: Optionally specify a slice name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available slices.

**Steps**

1. **Select the slice**

   If a name is provided, use it. Otherwise:

   - Infer from conversation context if the user mentioned a slice.
   - Run `specify status --format json` to enumerate active slices from the dashboard. If only one entry exists, auto-select it. If multiple, use the **AskQuestion tool** to let the user pick.

   Always announce: "Using slice: <name>" and how to override (e.g., `/spec:build <other>`).

2. **Read project config**

   Read `.specify/project.yaml` (use the Read tool) for `domain` and `rules`. These are constraints for you — do not copy them into code comments. Effective rules: for each brief ID under `rules`, use the project's value when present and non-empty; rules are optional overrides.

3. **Check lifecycle status and progress**

   Run `specify slice status <name> --format json`. Handle the reported `status`:

   - `defining`: warn that artifacts may be incomplete — some may not have been generated yet. Suggest running `/spec:define` to complete them. Optionally abort.
   - `complete`: congratulate, all tasks already done. Suggest `/spec:merge` and stop.
   - `merged` or `dropped`: tell the user the slice is terminal; ask if they want a new slice.
   - Otherwise (`defined`, `building`): proceed.

4. **Validate prerequisites (define artifacts + build-brief needs)**

   This step has two passes: an agent judgement pass over `tasks.md` (4a), then the deterministic CLI shape check (4b). Do them in order; do not skip 4a.

   **4a. Task preflight (agent judgement).** Read `.specify/slices/<name>/tasks.md` directly. For each checkbox line, confirm in context that:

   1. The action is executable by an agent in this repo using code, tooling, mocks, fixtures, contract validators, build commands, or one of the reviewer / writer skills available to the capability (consult the capability's `briefs/tasks.md` Available Skills table if unsure).
   2. The task does not require human-only action — manual app testing, real-world API credentials, visual inspection, physical-device-only checks, app store review, or asking the user to verify behavior. Read sentences in full: a task that says "without manual visual inspection" is *avoiding* the human action and is fine; a task that says "manually verify the iOS app against the real API" is requiring it and must be rewritten.
   3. The list as a whole includes at least one verification task — tests, fixture replay, contract verification, reviewer skill, or a build/check step.

   If any task fails (1) or (2), halt and ask the user to re-run `/spec:define` to rewrite the offending tasks. If (3) fails, halt and ask the user to re-run `/spec:define` to add a verification task. Do not attempt to rewrite `tasks.md` here — task authoring belongs to `/spec:define`, and per the Guardrails below `tasks.md` is not edited directly from this skill.

   **4b. Validate shape.** Run `specify slice validate <name> --format json`. Inspect the report:

   - If `passed` is `false` and the failures are about missing define-phase artifacts, halt and tell the user to run `/spec:define` to fill them in.
   - If `passed` is `false` for other reasons, report the details but ask the user whether to proceed.
   - If `passed` is `true`, continue.

   The CLI's `tasks.*` rules cover only deterministic shape (checkbox format, group headings); agent-completability is judged in 4a, not here.

   Supplement with `specify capability pipeline build --change .specify/slices/<name> --format json` if you need to inspect the build brief's `needs` and `path` directly (e.g. to tell the user which define artifact is missing).

5. **Read the build brief and supporting artifacts**

   `specify capability pipeline build --change .specify/slices/<name> --format json` returns the build brief's `path`, `needs`, and `tracks`. Read the brief body from that path; the build loop follows it step-by-step.

   Read the tracked tasks file (its path comes from `specify slice task progress` below) and every define-phase artifact you need for context (paths available via `specify capability pipeline define --change <slice-dir> --format json`).

6. **Show current progress**

   Run `specify slice task progress <name> --format json` for structured counts. See [output shape](../../../references/cli-output-shapes.md#specify-slice-task-progress).

   Display the summary and remaining tasks. If `pending` is zero, congratulate and suggest `/spec:merge`.

7. **Transition to building (first time only)**

   If the slice's current status is `defined`, run:

   ```bash
   specify slice transition <name> building --format json
   ```

   The CLI stamps `build-started-at` and enforces the `defined → building` edge. If the status is already `building`, skip this step. Never hand-edit `.metadata.yaml`.

8. **Implement tasks (loop until done or blocked)**

   For each pending task returned by `specify slice task progress`:

   - Inspect the task's `skill-directive` field. When present (`plugin`/`skill` pair), invoke that skill directly with the standard arguments (e.g. `/omnia:crate-writer $CRATE_PATH`). When absent, follow the build brief body's step-by-step execution (mode detection, verification loop, etc.).
   - Announce which task is being worked on.
   - Make the code changes required. Keep changes minimal and focused.
   - Mark the task complete: `specify slice task mark <name> <task-number> --format json`. The call is idempotent — a re-mark on an already-completed task is a no-op.
   - Continue to the next task.

   **Pause if:**

   - A task is unclear → ask for clarification.
   - Implementation reveals a design issue → suggest updating artifacts (`/spec:define <name> <artifact-id>` to regenerate). Do not transition status.
   - Error or blocker encountered → report and wait for guidance.
   - User interrupts.

9. **On completion or pause, show status**

   If all tasks are complete, transition to `complete`:

   ```bash
   specify slice transition <name> complete --format json
   ```

   The CLI stamps `completed-at` and enforces the `building → complete` edge.

   Display:

   - Tasks completed this session
   - Overall progress via `specify slice task progress <name>`
   - If all done: suggest `/spec:merge`
   - If paused: explain why and wait for guidance

**Output During Implementation**

```
## Implementing: <slice-name>

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

**Slice:** <slice-name>
**Progress:** 7/7 tasks complete

### Completed This Session
- [x] Task 1
- [x] Task 2
...

All tasks complete! Ready to merge this slice.
Run `/spec:merge` to finalize.
```

**Output On Pause (Issue Encountered)**

```
## Implementation Paused

**Slice:** <slice-name>
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

- **All artifacts live under `.specify/slices/<name>/`**. Read and write artifacts relative to this directory only.
- Keep going through tasks until done or blocked.
- Always read context files before starting.
- If a task is ambiguous, pause and ask before implementing.
- If implementation reveals issues, pause and suggest artifact updates.
- Keep code changes minimal and scoped to each task.
- Flip task checkboxes through `specify slice task mark`; do not edit `tasks.md` directly.
- Route `.metadata.yaml` writes through `specify slice transition` — see [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state).
- Pause on errors, blockers, or unclear requirements — don't guess.

**Fluid Workflow Integration**

This skill supports the "actions on a slice" model:

- **Can be invoked anytime**: Before all artifacts are done (if tasks exist), after partial implementation, interleaved with other actions.
- **Allows artifact updates**: If implementation reveals design issues, suggest updating artifacts -- not phase-locked, work fluidly.

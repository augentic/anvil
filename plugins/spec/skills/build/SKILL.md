---
name: specify-build
description: Implement tasks from a Specify slice. Use when a slice already exists with pending tasks and needs implementation work, not when first authoring the slice (`define`) or finalizing it (`merge`).
argument-hint: "[slice-name]"
---

# Build skill

Implement tasks from a Specify slice. Deterministic bookkeeping — slice selection, lifecycle transitions, capability resolution, brief completion checks, task progress counting, checkbox flips — is delegated to the `specify` CLI. This skill drives the agent-side work: reading the build brief body, dispatching skill directives, and making code changes.

When working plan-driven (a `plan.yaml` exists), the corresponding entry should already be `in-progress`. `/spec:build` does not touch `plan.yaml`; plan-status transitions remain with the driver or human loop named in the shared [phase outcome contract](../../references/phase-outcome-contract.md).

## Critical Path

### 1. Select the slice and read project config

If a slice name was supplied, use it. Otherwise infer from conversation context, or run `specify status --format json` to enumerate active slices from the dashboard. If only one entry exists, auto-select it; if multiple, use the **AskQuestion tool** to let the user pick. Always announce: "Using slice: <name>" and how to override (e.g., `/spec:build <other>`).

Then read `.specify/project.yaml` for `domain` and `rules`. These are constraints for the agent — do not copy them into code comments. For each brief ID under `rules`, use the project's value when present and non-empty; rules are optional overrides.

### 2. Check lifecycle status

Run `specify slice status <name> --format json` and branch on the reported `status`:

- `defining` — warn that artifacts may be incomplete. Suggest running `/spec:define` to complete them. Optionally abort.
- `complete` — congratulate, all tasks are done. Suggest `/spec:merge` and stop.
- `merged` or `dropped` — the slice is terminal; ask if they want a new slice.
- `defined` or `building` — proceed.

### 3. Validate prerequisites

Two passes: an agent judgement pass over `tasks.md` (3a), then the deterministic CLI shape check (3b). Do them in order; do not skip 3a.

**3a. Task preflight (agent judgement).** Read `.specify/slices/<name>/tasks.md` directly. For each checkbox line, confirm in context that:

1. The action is executable by an agent in this repo using code, tooling, mocks, fixtures, contract validators, build commands, or one of the reviewer / writer skills available to the capability (consult the capability's `briefs/tasks.md` Available Skills table if unsure).
2. The task does not require human-only action — manual app testing, real-world API credentials, visual inspection, physical-device-only checks, app-store review, or asking the user to verify behavior. Read sentences in full: a task that says "without manual visual inspection" is *avoiding* the human action and is fine; a task that says "manually verify the iOS app against the real API" is requiring it and must be rewritten.
3. The list as a whole includes at least one verification task — tests, fixture replay, contract verification, reviewer skill, or a build/check step.

If any task fails (1) or (2), halt and ask the user to re-run `/spec:define` to rewrite the offending tasks. If (3) fails, halt and ask the user to re-run `/spec:define` to add a verification task. Do not attempt to rewrite `tasks.md` here — task authoring belongs to `/spec:define`, and per the Guardrails below `tasks.md` is not edited directly from this skill.

**3b. Validate shape.** Run `specify slice validate <name> --format json`. If `passed` is `false` and the failures are about missing define-phase artifacts, halt and tell the user to run `/spec:define` to fill them in. If `passed` is `false` for other reasons, report the details but ask the user whether to proceed. Otherwise continue. The CLI's `tasks.*` rules cover only deterministic shape; agent-completability is judged in 3a.

Supplement with `specify capability pipeline build --change .specify/slices/<name> --format json` if you need to inspect the build brief's `needs` and `path` directly (e.g., to tell the user which define artifact is missing).

### 4. Read the build brief and show progress

`specify capability pipeline build --change .specify/slices/<name> --format json` returns the build brief's `path`, `needs`, and `tracks`. Read the brief body from that path; the build loop follows it step by step. Read the tracked tasks file (its path comes from `specify slice task progress`) and every define-phase artifact you need for context.

Run `specify slice task progress <name> --format json` for structured counts (see [output shape](../../../references/cli-output-shapes.md#specify-slice-task-progress)) and display the summary and remaining tasks. If `pending` is zero, congratulate and suggest `/spec:merge`.

### 5. Transition to building (first time only)

If the slice's current status is `defined`, run `specify slice transition <name> building --format json`. The CLI stamps `build-started-at` and enforces the `defined → building` edge. If status is already `building`, skip this step. Never hand-edit `.metadata.yaml`.

### 6. Implement tasks (loop until done or blocked)

For each pending task returned by `specify slice task progress`:

- Inspect the task's `skill-directive` field. When present (`plugin` / `skill` pair), invoke that skill directly with the standard arguments (e.g. `/omnia:crate-writer $CRATE_PATH`). When absent, follow the build brief body's step-by-step execution (mode detection, verification loop, etc.).
- Announce which task is being worked on.
- Make the code changes required. Keep changes minimal and focused.
- Mark the task complete: `specify slice task mark <name> <task-number> --format json`. The call is idempotent — a re-mark on an already-completed task is a no-op.
- Continue to the next task.

**Pause if:**

- A task is unclear → ask for clarification.
- Implementation reveals a design issue → suggest updating artifacts (`/spec:define <name> <artifact-id>` to regenerate). Do not transition status.
- Error or blocker encountered → report and wait for guidance.
- User interrupts.

### 7. On completion or pause, show status

If all tasks are complete, transition to `complete` with `specify slice transition <name> complete --format json`. The CLI stamps `completed-at` and enforces the `building → complete` edge. Display:

- Tasks completed this session
- Overall progress via `specify slice task progress <name>`
- If all done: suggest `/spec:merge`
- If paused: explain why and wait for guidance

Output templates for the per-session implementation, completion, and pause summaries live in [`references/output-templates.md`](references/output-templates.md).

## Phase outcome contract

This skill is the **build** phase of the `/change:execute` driver loop. Apply the shared [phase outcome contract](../../references/phase-outcome-contract.md), including build's per-phase deltas, journal rules, plan-mutation allowlist, and verbatim-`summary` rule.

## Input

Optionally specify a slice name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available slices.

## Guardrails

- **All artifacts live under `.specify/slices/<name>/`**. Read and write artifacts relative to this directory only.
- Keep going through tasks until done or blocked.
- Always read context files before starting.
- If a task is ambiguous, pause and ask before implementing.
- If implementation reveals issues, pause and suggest artifact updates.
- Keep code changes minimal and scoped to each task.
- Flip task checkboxes through `specify slice task mark`; do not edit `tasks.md` directly.
- Route `.metadata.yaml` writes through `specify slice transition` — see [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state).
- Pause on errors, blockers, or unclear requirements — don't guess.

## Fluid workflow integration

This skill supports the "actions on a slice" model:

- **Can be invoked anytime**: before all artifacts are done (if tasks exist), after partial implementation, interleaved with other actions.
- **Allows artifact updates**: if implementation reveals design issues, suggest updating artifacts — not phase-locked, work fluidly.

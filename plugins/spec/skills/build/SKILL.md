---
name: build
description: Implement tasks from a Specify change. Use when the user wants to start implementing, continue implementation, or work through tasks.
license: MIT
argument-hint: "[change-name?]"
---

Implement tasks from a Specify change.

Deterministic bookkeeping — change selection, lifecycle transitions, schema
resolution, brief completion checks, task progress counting, checkbox flips —
is delegated to the `specify` CLI. This skill drives the agent-side work:
reading the build brief body, dispatching skill directives, and making code
changes.

When working plan-driven (a `.specify/plan.yaml` exists), the corresponding plan entry should already be `in-progress` — the human runs `specify initiative transition <name> in-progress` once before `/spec:build` starts. `/spec:build` itself does not touch `plan.yaml`; the plan transition out of `in-progress` happens from `/spec:merge` (→ `done`) or `/spec:drop` (→ `failed` / `blocked`).

> See `rfcs/archive/rfc-2-execution.md` §"Execution Model Overview" and
> `rfcs/assets/execution.png` for where this skill sits in the
> `/spec:execute` driver loop.

## Phase outcome contract (RFC-2 §"Phase Outcome Contract")

This skill is the **build** phase of the `/spec:execute` driver loop.
Before returning control to the caller, always record the phase's outcome
via:

```bash
specify change phase-outcome <name> build <outcome> --summary "..." [--context "..."]
```

where `<outcome>` is exactly one of:

- `success`  — every build brief produced its `generates` artefacts, the
  verify-repair loop converged, and `specify task progress` reports
  `pending == 0`. The change is ready for `/spec:merge`.
- `failure`  — a brief failed after the repair budget was exhausted
  (e.g. the Omnia `build.md` 3-iteration verify-repair loop could not
  converge; a specialist writer skill returned a non-recoverable error).
  Use `--summary` to name which brief and the load-bearing stderr/test
  line; use `--context` for verbatim detail (failing test name, compiler
  error tail, etc.).
- `deferred` — human judgement is needed (task is ambiguous,
  implementation reveals a design issue that must be resolved before
  coding, artefact updates are required but not safe to do
  unattended). Use `--summary` to name the question.

`/spec:execute` reads `.specify/changes/<name>/.metadata.yaml:outcome`
on return and translates the outcome into a plan transition
(`done` / `failed` / `blocked`). If the field is missing or malformed,
`/spec:execute` treats the phase as `deferred` and stops for triage —
do not skip the CLI call. This `phase-outcome` invocation is the
**last action** the skill takes before returning control.

## Journal entries during the run (RFC-2 §"Question Recording")

Whenever the skill encounters a situation the human should see — a
genuine question, a repair attempt that failed, or a notable recovery —
append to `.specify/changes/<name>/journal.yaml` **during** the run,
not just at the end:

```bash
specify change journal-append <name> build <kind> --summary "..." [--context "..."]
```

Kinds:

- `question` — task is ambiguous, implementation reveals a design
  issue, or anything that might produce a `deferred` outcome at the end
  of the phase. Write one entry per question so the human sees the full
  trail when triaging.
- `failure` — a brief (or its specialist writer) returned an error
  after retry. Write one entry per failure; the final `phase-outcome`
  summary rolls up only the load-bearing one, but auditors still see
  every attempt inside the verify-repair loop.
- `recovery` — a self-heal / recovery step happened. (Typically written
  by `/spec:execute` itself; phases rarely need to append this kind.)

`journal.yaml` is a pure append-only audit log; `/spec:execute` never
consumes it as a signalling channel. The `outcome` field in
`.metadata.yaml` is the only state `/spec:execute` reads on phase
return.

## Mutating the plan mid-run (RFC-2 §"Phase Boundary → Rule 2")

Phases may shell out to `specify initiative create` / `specify initiative amend`
mid-run when they discover something structural about the initiative.
Both commands write `.specify/plan.yaml` synchronously — the new or
updated entry is visible to every subsequent `/spec:execute` iteration.

Allowed:

- `specify initiative create <new-name> --affects <current-name> --description "..."`
  when implementation uncovers a neighbouring defect or a prerequisite
  refactor that warrants its own change.
- `specify initiative amend <current-name> --depends-on <newly-needed>` when
  the phase discovers a dependency on another plan entry. `amend` may
  target the currently-active entry — non-`status` fields on an
  `in-progress` entry are fair game.

Forbidden:

- Writing `status` through `amend`. The `PlanChangePatch` type has no
  `status` field — this is a type-system guarantee. Status transitions
  are `/spec:execute`'s sole prerogative via `specify initiative transition`.
- Hand-editing `.specify/plan.yaml` or
  `.specify/changes/<name>/.metadata.yaml`. Always route through the
  CLI so the single-writer invariant in RFC-2 §"Plan Mutation and
  Crash Safety" holds.

**Input**: Optionally specify a change name. If omitted, check if it can be inferred from conversation context. If vague or ambiguous you MUST prompt for available changes.

**Steps**

1. **Select the change**

   If a name is provided, use it. Otherwise:

   - Infer from conversation context if the user mentioned a change.
   - Run `specify status --format json` to enumerate active changes. If only one entry exists, auto-select it. If multiple, use the **AskQuestion tool** to let the user pick.

   Always announce: "Using change: <name>" and how to override (e.g., `/spec:build <other>`).

2. **Read project config**

   Read `.specify/project.yaml` (use the Read tool) for `domain` and `rules`. These are constraints for you — do not copy them into code comments. Effective rules: for each brief ID under `rules`, use the project's value when present and non-empty; rules are optional overrides.

3. **Check lifecycle status and progress**

   Run `specify change status <name> --format json`. Handle the reported `status`:

   - `defining`: warn that artifacts may be incomplete — some may not have been generated yet. Suggest running `/spec:define` to complete them. Optionally abort.
   - `complete`: congratulate, all tasks already done. Suggest `/spec:merge` and stop.
   - `merged` or `dropped`: tell the user the change is terminal; ask if they want a new change.
   - Otherwise (`defined`, `building`): proceed.

4. **Validate prerequisites (define artifacts + build-brief needs)**

   Run `specify validate .specify/changes/<name> --format json`. Inspect the report:

   - If `passed` is `false` and the failures are about missing define-phase artifacts, halt and tell the user to run `/spec:define` to fill them in.
   - If `passed` is `false` for other reasons, report the details but ask the user whether to proceed.
   - If `passed` is `true`, continue.

   Supplement with `specify schema pipeline build --change .specify/changes/<name> --format json` if you need to inspect the build brief's `needs` and `path` directly (e.g. to tell the user which define artifact is missing).

5. **Read the build brief and supporting artifacts**

   `specify schema pipeline build --change .specify/changes/<name> --format json` returns the build brief's `path`, `needs`, and `tracks`. Read the brief body from that path; the build loop follows it step-by-step.

   Read the tracked tasks file (its path comes from `specify task progress` below) and every define-phase artifact you need for context (paths available via `specify schema pipeline define --change <change-dir> --format json`).

6. **Show current progress**

   Run `specify task progress .specify/changes/<name> --format json` for structured counts:

   ```json
   {"total": 7, "complete": 2, "pending": 5, "tasks": [{"number": "1.1", ...}]}
   ```

   Display the summary and remaining tasks. If `pending` is zero, congratulate and suggest `/spec:merge`.

7. **Transition to building (first time only)**

   If the change's current status is `defined`, run:

   ```bash
   specify change transition <name> building --format json
   ```

   The CLI stamps `build-started-at` and enforces the `defined → building` edge. If the status is already `building`, skip this step. Never hand-edit `.metadata.yaml`.

8. **Implement tasks (loop until done or blocked)**

   For each pending task returned by `specify task progress`:

   - Inspect the task's `skill-directive` field. When present (`plugin`/`skill` pair), invoke that skill directly with the standard arguments (e.g. `/omnia:crate-writer $CRATE_PATH`). When absent, follow the build brief body's step-by-step execution (mode detection, verification loop, etc.).
   - Announce which task is being worked on.
   - Make the code changes required. Keep changes minimal and focused.
   - Mark the task complete: `specify task mark .specify/changes/<name> <task-number> --format json`. The call is idempotent — a re-mark on an already-completed task is a no-op.
   - Continue to the next task.

   **Pause if:**

   - A task is unclear → ask for clarification.
   - Implementation reveals a design issue → suggest updating artifacts (`/spec:define <name> <artifact-id>` to regenerate). Do not transition status.
   - Error or blocker encountered → report and wait for guidance.
   - User interrupts.

9. **On completion or pause, show status**

   If all tasks are complete, transition to `complete`:

   ```bash
   specify change transition <name> complete --format json
   ```

   The CLI stamps `completed-at` and enforces the `building → complete` edge.

   Display:

   - Tasks completed this session
   - Overall progress via `specify task progress .specify/changes/<name>`
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

- **All artifacts live under `.specify/changes/<name>/`**. Read and write artifacts relative to this directory only.
- Keep going through tasks until done or blocked.
- Always read context files before starting.
- If a task is ambiguous, pause and ask before implementing.
- If implementation reveals issues, pause and suggest artifact updates.
- Keep code changes minimal and scoped to each task.
- Flip task checkboxes through `specify task mark`; do not edit `tasks.md` directly.
- Never hand-edit `.metadata.yaml`. All status transitions go through `specify change transition`; the CLI enforces the legal set of lifecycle values.
- Pause on errors, blockers, or unclear requirements — don't guess.

**Fluid Workflow Integration**

This skill supports the "actions on a change" model:

- **Can be invoked anytime**: Before all artifacts are done (if tasks exist), after partial implementation, interleaved with other actions.
- **Allows artifact updates**: If implementation reveals design issues, suggest updating artifacts -- not phase-locked, work fluidly.

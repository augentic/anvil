# Specify Skills Reference

Specify skills are organised into three layers that mirror the runtime execution stack. Layer 1 is the CLI primitives everything else builds on. Layer 2 skills operate on a single change. Layer 3 skills orchestrate multi-change initiatives.

## Layer 1 — CLI primitives

All skills delegate deterministic operations to the `specify` CLI. The two primary command groups are:

- `**specify change …**` — create, inspect, transition, and archive individual changes.
- `**specify initiative …**` — scaffold, populate, validate, transition, and archive an initiative plan.

Skills never hand-edit `.metadata.yaml`, `plan.yaml`, or the archive directory. Every write routes through the CLI, which enforces lifecycle rules and validates inputs in one place.

---

## Layer 2 — Change lifecycle

These skills operate on a single change inside `.specify/changes/<name>/`. They form the define → build → merge loop.

### `/spec:define`

Create a new change and generate all artifacts in one step.

```
/spec:define [description] [artifact-id?] [--source <key>=<path-or-url>...]
```

**When to use:** You have a clear idea of what to build and want a complete proposal, specs, design, and tasks ready for implementation. Also used to regenerate a single artifact for an existing change.

**Artifacts produced:**


| Artifact                   | Location                                        | Content                                         |
| -------------------------- | ----------------------------------------------- | ----------------------------------------------- |
| `proposal.md`              | `.specify/changes/<name>/proposal.md`           | Why the change exists and what is in scope      |
| `spec.md` (per capability) | `.specify/changes/<name>/specs/<crate>/spec.md` | Behavioral requirements with BDD scenarios      |
| `design.md`                | `.specify/changes/<name>/design.md`             | Domain model, APIs, integrations, configuration |
| `tasks.md`                 | `.specify/changes/<name>/tasks.md`              | Implementation task list with checkboxes        |


**Lifecycle:** Creates the change directory, generates artifacts via the schema's `pipeline.define` briefs in dependency order, scans `touched-specs`, and transitions the change to `defined`.

---

### `/spec:build`

Implement tasks from a defined change.

```
/spec:build [change-name?]
```

**When to use:** A change is `defined` (all artifacts present) and you want to start or continue implementation.

**Artifacts produced:** Source code changes in the project codebase (not under `.specify/`). Task checkboxes in `tasks.md` are flipped via `specify task mark` as each task completes.

**Lifecycle:** Transitions the change from `defined` to `building`. Reads the build brief and works through tasks sequentially. Delegates to specialist skills (e.g. `/omnia:crate-writer`, `/vectis:core-writer`) when a task carries a skill directive. On completion of all tasks, transitions to `complete`.

---

### `/spec:merge`

Merge a completed change into the baseline.

```
/spec:merge [change-name?]
```

**When to use:** All tasks are complete and you want to finalize the change.

**Artifacts produced:**


| Artifact              | Location                              | Content                                        |
| --------------------- | ------------------------------------- | ---------------------------------------------- |
| Merged baseline specs | `.specify/specs/<capability>/spec.md` | Updated or new baseline spec files             |
| Archived change       | `.specify/archive/YYYY-MM-DD-<name>/` | The full change directory, preserved for audit |


**Lifecycle:** Previews the merge via `specify spec preview`, checks for baseline drift via `specify spec conflict-check`, confirms with the user, then runs `specify merge` which applies deltas, validates coherence, transitions to `merged`, and moves the change to the archive.

---

### `/spec:drop`

Discard a change without merging specs.

```
/spec:drop [change-name?] [--reason "<rationale>"]
```

**When to use:** A change should not be merged — it was exploratory, superseded, or blocked.

**Artifacts produced:**


| Artifact        | Location                              | Content                                         |
| --------------- | ------------------------------------- | ----------------------------------------------- |
| Archived change | `.specify/archive/YYYY-MM-DD-<name>/` | The full change directory with `dropped` status |


**Lifecycle:** Confirms with the user (unless `--reason` is supplied for non-interactive use), then runs `specify change drop` which transitions to `dropped` and archives the directory. Baseline specs remain unchanged.

---

### `/spec:extract`

Extract Specify artifacts from existing source code.

```
/spec:extract <source-path> <change-dir> [--include <glob>...] [--exclude <glob>...] [--manifest <path>]
```

**When to use:** You have an existing codebase and want to produce reconstruction-grade, language-agnostic specs and design from its source code. Typically run during `/spec:define` for migration initiatives, or standalone after `/spec:init` for brownfield projects.

**Artifacts produced:**


| Artifact                   | Location                             | Content                                                    |
| -------------------------- | ------------------------------------ | ---------------------------------------------------------- |
| `spec.md` (per capability) | `<change-dir>/specs/<crate>/spec.md` | Requirements with BDD scenarios extracted from source      |
| `design.md`                | `<change-dir>/design.md`             | Domain model, APIs, dependencies, business logic with tags |


**Key principle:** Artifacts are language-agnostic — they describe what the code does, not how it should be reimplemented.

---

### `/spec:status`

Show the current state of active changes.

```
/spec:status [change-name?]
```

**When to use:** You want to check where things stand — which changes are active, what artifacts are complete, how many tasks remain.

**Artifacts produced:** None (read-only). Renders a summary of active changes with per-brief artifact completion, task progress, lifecycle status, and next-step guidance.

---

### `/spec:verify`

Detect drift between code and baseline specs.

```
/spec:verify [capability-name?]
```

**When to use:** You want to check whether the codebase still matches the merged specifications. Useful before merging a new change, or periodically to catch undocumented changes.

**Artifacts produced:** None (read-only). Produces a drift report classifying each baseline requirement as COVERED, DRIFTED, MISSING, or UNSPECIFIED, with suggested actions.

---

### `/spec:explore`

A thinking partner for ideas, investigation, and requirements.

```
/spec:explore [change-name?]
```

**When to use:** You want to think through a problem, explore options, investigate the codebase, or clarify requirements before or during a change. There is no fixed workflow — it follows the conversation.

**Artifacts produced:** None by default. May update existing change artifacts (proposal, specs, design) if the user asks to capture a decision. Never writes application code.

---

## Layer 3 — Initiative orchestration

These skills coordinate multi-change programs through `.specify/plan.yaml`.

### `/spec:plan`

Author `plan.yaml` for a new initiative.

```
/spec:plan <initiative-name> \
    [--from <path>...]          # documentation inputs
    [--against <path>]          # existing codebase to delta against
    [--source <key>=<path>...]  # named legacy-code sources
    [--focus <area>]            # scoping hint
    [--extend]                  # add to existing plan
    [--dry-run]                 # preview only, write nothing
```

**When to use:** You have a body of work (migration, new feature set, modernisation) that will span multiple changes and you want a structured plan with dependency ordering.

**Artifacts produced:**


| Artifact                    | Location                                            | Content                                           |
| --------------------------- | --------------------------------------------------- | ------------------------------------------------- |
| `plan.yaml`                 | `.specify/plan.yaml`                                | Ordered change list with dependencies and status  |
| `discovery.md`              | `.specify/plans/<name>/discovery.md`                | Capability inventory from input analysis          |
| `proposal.md`               | `.specify/plans/<name>/proposal.md`                 | Audit trail of slice accept/edit/reject decisions |
| `workspace.md` (multi-repo) | `.specify/plans/<name>/workspace.md`                | Peer inventory for cross-repo planning            |
| Structural metadata         | `.specify/plans/<name>/analyze/<key>/metadata.json` | Source-tree facts (language, LOC, modules)        |


**Core loop:** Parse inputs → scaffold plan via `specify initiative init` → run discovery (via `/spec:analyze`) → optionally sync peers → interactively propose slices → validate → hand off.

---

### `/spec:execute`

Drive an initiative through its plan, automating define → build → merge.

```
/spec:execute              # run one change, stop
/spec:execute --dry-run    # preview next change + progress
/spec:execute --loop       # run until no eligible change remains
```

**When to use:** A `plan.yaml` exists and you want to automate the change-by-change execution loop instead of driving it manually.

**Artifacts produced:** No artifacts of its own. Invokes `/spec:define`, `/spec:build`, `/spec:merge` (and `/spec:drop` on failure) for each change. Writes plan entry transitions via `specify initiative transition`. Manages `.specify/plan.lock` for concurrency safety.

**Per-change algorithm:** Pick next eligible entry → transition to `in-progress` → define → build → merge → read phase outcome → transition to `done` / `failed` / `blocked`. Self-heals on startup if a prior run crashed mid-change.

---

### `/spec:analyze`

Plan-time capability inference (used internally by `/spec:plan`).

```
/spec:analyze <input-path> <output-dir> --kind <legacy-code|documentation> [--source-key <k>]
```

**When to use:** Typically invoked by the discovery brief during `/spec:plan`, not directly. Reads one input (a code tree or documentation bundle) and appends capability summaries to `discovery.md`.

**Artifacts produced:**


| Artifact             | Location                                   | Content                                                              |
| -------------------- | ------------------------------------------ | -------------------------------------------------------------------- |
| Capability summaries | `<output-dir>/discovery.md` (appended)     | Per-capability name, summary, source files, dependencies, confidence |
| Structural metadata  | `<output-dir>/analyze/<key>/metadata.json` | Language, LOC, module count (legacy-code only)                       |


**Key principle:** Produces capability summaries, not full specs. Deep extraction happens per-slice at define time via `/spec:extract`.

---

## Setup

### `/spec:init`

Initialize Specify in a project.

```
/spec:init [schema?]
```

**When to use:** Once per project, before any other `/spec:` skill.

**Artifacts produced:**


| Artifact            | Location                            | Content                       |
| ------------------- | ----------------------------------- | ----------------------------- |
| Project config      | `.specify/project.yaml`             | Schema, domain, rules         |
| Schema cache        | `.specify/.cache/<schema>/`         | Cached schema and brief files |
| Directory structure | `.specify/{changes,specs,archive}/` | Empty scaffold                |


Detects existing codebases and offers to create an `initial-baseline` change for `/spec:extract`.

---

## Typical workflows

**Single change (manual):**

```
/spec:init  →  /spec:define  →  /spec:build  →  /spec:merge
```

**Multi-change initiative (automated):**

```
/spec:plan <name> --source legacy=./path  →  /spec:execute --loop
```

**Brownfield onboarding:**

```
/spec:init  →  /spec:extract . .specify/changes/initial-baseline/  →  /spec:merge initial-baseline
```

**Thinking first:**

```
/spec:explore  →  (when ready)  →  /spec:define
```


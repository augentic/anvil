# RFC-2 Addendum: Manifest Orchestrator Design

> Companion to [RFC-2: Manifests](rfc-2-manifests.md).

## Abstract

The manifest orchestrator is a non-interactive skill that reads `manifest.yaml`, selects the next eligible change, runs the appropriate step sequence, and updates the manifest — recording questions and failures rather than blocking on them.

It is the only skill that calls other skills programmatically. All existing skills (extract, define, build, merge, drop) remain unchanged; the orchestrator invokes them with arguments and interprets their outputs.

## Invocation

```
/spec:orchestrate [--dry-run] [--loop] [manifest-path?]
```

- No arguments: reads `.specify/manifest.yaml`, processes a single change then stops (supervised mode)
- `--loop`: process changes until no eligible changes remain (autonomous mode)
- `--dry-run`: show what would run next without executing

## Core Loop

```text
  1. Read manifest.yaml
  2. Select next eligible change (all depends-on are done, status is pending)
  3. If none eligible → stop (report blocked/remaining counts)
  4. Transition manifest entry: pending → in-progress
  5. Run the step sequence (define → build → merge, with field-presence adjustments)
  6. On success: transition in-progress → done
  7. On failure: drop the Specify change, transition in-progress → failed, record reason
  8. On deferred question: drop the Specify change, transition in-progress → blocked, record question
  9. If --loop: continue from step 1; otherwise stop
```

## Non-Interactive Execution (Issue 2a)

### Problem

The existing skills use `AskQuestion` for confirmations, disambiguation, and warnings. An automated loop can't stop and wait for human input on every change.

### Design

The orchestrator invokes each skill with pre-resolved arguments that eliminate the need for interactive prompts. Each skill's interactive decision points are handled as follows:

| Skill | Interactive Point | Orchestrator Strategy |
|---|---|---|
| **define** | "What do you want to build?" | Pre-supplied: change `name` + `description` from manifest |
| **define** | "Change already exists — continue or restart?" | Pre-resolved: if a prior Specify change exists for this manifest entry and its status is `defining`, continue; otherwise create fresh |
| **define** | Overlapping `touched_specs` warning | Logged to change journal, not blocking (already informational) |
| **extract** | Source path confirmation | Pre-supplied: resolved from manifest `sources` map |
| **build** | "Task is unclear" pause | Recorded as a question, change deferred |
| **build** | "Design issue discovered" pause | Recorded as a question, change deferred |
| **merge** | "Confirm change before merging" | Pre-confirmed: orchestrator only calls merge after build completes |
| **merge** | "Incomplete tasks — proceed?" | Never reached: orchestrator only merges when build status is `complete` |
| **merge** | "Baseline conflict detected" | Recorded as a question, change deferred |
| **drop** | "Confirm before dropping" | Pre-confirmed: orchestrator only drops on failure/deferral |

Skills don't need non-interactive variants. The orchestrator supplies deterministic answers to every decision point that it can resolve from the manifest and artifact state. When it *can't* resolve a decision (a genuine question requiring human judgement), it defers the change rather than guessing.

### Question Recording

When a step encounters a situation requiring human input, the orchestrator writes a structured entry to a journal file at `.specify/changes/<name>/journal.yaml`:

```yaml
entries:
  - timestamp: 2026-04-16T14:30:00Z
    step: build
    type: question
    summary: "Task 3/7 unclear — payment gateway contract references undefined type PaymentIntent"
    context: |
      Working on task: Implement checkout payment processing
      The spec references PaymentIntent but no type definition exists in
      design.md or upstream specs.
```

The manifest entry transitions to `blocked` with a `description` summarizing the question. This reuses the existing `blocked` status and its manual `blocked → pending` transition — a human reviews the journal, resolves the question (perhaps by updating the manifest description, adding to the spec, or refining the design), and unflags the change.

## Failure and Resumption (Issue 2b)

### Problem

A change can fail mid-build (tests don't pass, extraction produces garbage, merge conflicts). What happens to the half-created Specify change?

### Design

Mark as `failed` with the reason and move on to the next eligible change.

```text
on failure at any step:
  1. Record failure reason in journal.yaml:
     - timestamp, step, type: failure, summary, context (stderr, test output, etc.)
  2. Drop the Specify change via /spec:drop (archives partial artifacts)
  3. Transition manifest entry: in-progress → failed
  4. Set failure-reason on the manifest entry from the summary
  5. Continue to next eligible change
```

**Retry**: A human reviews the failure, optionally updates the manifest entry's description or dependencies, then transitions `failed → pending`. On the next orchestrator run, a fresh Specify change is created for that entry. The archived dropped change provides forensic context.

### Failure vs Deferral

Failure means the step ran and produced an error. Deferral means the step couldn't proceed without human input. Both result in the Specify change being dropped and archived, but the distinction matters for triage:

| | Manifest status | Cause | Resolution |
|---|---|---|---|
| Failure | `failed` | Step error (tests, merge conflict, bad extraction) | Fix the issue, retry (`failed → pending`) |
| Deferral | `blocked` | Needs human decision (ambiguous requirement, design question) | Answer the question, unflag (`blocked → pending`) |

## Context Threading (Issue 2c)

### Problem

How does the orchestrator pass context between extract → define → build → merge for a single change?

### Design

The artifacts are the context. There is no separate context object or state bag. Each step reads what the previous step wrote:

```text
┌──────────────────────────────────────────────────────────────────┐
│                                                                  │
│  manifest.yaml         ← orchestrator reads: name, description,  │
│                           sources, affects, depends-on            │
│                                                                  │
│  /spec:extract                                                   │
│    reads: source repos (from manifest sources map)               │
│    writes: .specify/changes/<name>/specs/, design.md             │
│                                                                  │
│  /spec:define                                                    │
│    reads: extracted artifacts (or creates from description)      │
│    writes: proposal, specs, design, tasks per schema pipeline    │
│                                                                  │
│  /spec:build                                                     │
│    reads: all define artifacts + baseline specs                  │
│    writes: code, marks tasks complete                            │
│                                                                  │
│  /spec:merge                                                     │
│    reads: completed change artifacts + baseline                  │
│    writes: merged baseline specs, archives change                │
│                                                                  │
│  manifest.yaml         ← orchestrator writes: status → done      │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

The orchestrator's responsibility is limited to:

1. **Supplying initial arguments** to each skill invocation — derived from the manifest entry (name, source paths, description)
2. **Checking preconditions** between steps — reading `.metadata.yaml` to confirm the previous step completed (status progressed to the expected value)
3. **Deciding what to run next** — field presence (`sources`, `affects`) determines what context to pass to define; the Specify `LifecycleStatus` determines where to resume if re-entering a partially-completed change

### Resumption Within a Change

The existing `LifecycleStatus` values already encode which step ran last. The orchestrator uses this for resumption:

```text
match change.lifecycle_status:
  None           → start from the beginning (define, which invokes extract if sources present)
  defining       → resume/restart define
  defined        → start build
  building       → resume build
  complete       → run merge
```

If the orchestrator crashes mid-change and is restarted, it picks up where it left off by reading the manifest (which change is `in-progress`) and the Specify change's `.metadata.yaml` (which step was last completed).

## Step Sequence by Field Presence

The loop is always define → build → merge. The orchestrator adjusts what it passes to define based on which fields are present on the manifest entry:

```text
change with sources:    define (with extract) → build → merge
change with affects:    define (delta against affected specs) → build → merge
change (greenfield):    define → build → merge
```

These are not mutually exclusive — a change could have both `sources` and `affects`. The orchestrator passes both signals to define:

- **`sources`** present: the orchestrator resolves source paths from the top-level `sources` map and supplies them so define invokes `/spec:extract` for source analysis.
- **`affects`** present: the orchestrator passes the list of affected capability names so define loads the corresponding baseline specs as delta targets.

## Manifest Mutation and Crash Safety

The orchestrator owns all manifest writes. Status transitions happen at well-defined points:

1. `pending → in-progress`: **before** the first skill invocation for that change
2. `in-progress → done`: **after** `/spec:merge` completes successfully
3. `in-progress → failed`: **after** `/spec:drop` completes
4. `in-progress → blocked`: **after** `/spec:drop` completes and question is journaled

If the orchestrator crashes between merge and manifest update, the manifest shows `in-progress` but the Specify change is already `merged` and archived. On restart, the orchestrator detects the mismatch (no active Specify change for the `in-progress` manifest entry, but an archived `merged` change exists) and corrects the manifest to `done`.

## Skill Invocation Model

The orchestrator runs within the same agent session and invokes skills by their standard mechanism (e.g., `/spec:define change-name`). By default it processes a single change and stops, keeping the human in the loop. With `--loop`, it holds the agent for the duration of the initiative (or until all eligible changes are processed).

## Output and Observability

The orchestrator produces structured output at each transition:

```text
## Manifest Orchestrator

### Initiative: platform-v2
Progress: 3/10 changes done, 1 blocked, 6 pending

---

### Processing: email-verification (sources: [monolith])

Step 1/4: extract
  Source: /path/to/legacy-codebase
  Artifacts: specs/email-verification/spec.md, design.md ✓

Step 2/4: define
  Artifacts: proposal.md, specs, design.md, tasks.md ✓

Step 3/4: build
  Tasks: 5/5 complete ✓

Step 4/4: merge
  Baseline updated: .specify/specs/email-verification/spec.md ✓
  Status: done

---

### Next: registration-duplicate-email-crash (affects: [user-registration])
```

For deferred changes:

```text
### Processing: notification-preferences (greenfield)

Step 2/4: define
  ⚠ Question recorded — change deferred to blocked

  Question: The description says "notification channel and frequency settings"
  but doesn't specify which channels are in scope. The baseline has no
  notification infrastructure to reference.

  Journal: .specify/changes/notification-preferences/journal.yaml
  Action needed: Update the manifest description with channel scope, then
  unflag (blocked → pending) to retry.

### Skipping to next eligible change...
```

## Summary

| Concern | Resolution |
|---|---|
| Interactive skills | Orchestrator pre-resolves arguments; genuine questions defer the change |
| Failure | Drop the Specify change, mark `failed` with reason, advance |
| Resumption | Manifest `in-progress` + Specify `LifecycleStatus` encode exactly where to resume |
| Context threading | Artifacts written by each step are read by the next; manifest supplies initial args |
| Crash safety | Orchestrator detects manifest/change state mismatches on restart and self-heals |
| Observability | Structured output per step + `journal.yaml` for questions/failures |

The orchestrator adds one new file (`journal.yaml` per change) and no new manifest statuses — it works entirely within the existing status state machine and Specify lifecycle.

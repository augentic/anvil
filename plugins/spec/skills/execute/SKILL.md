---
name: execute
description: |
  Drive an initiative through its plan.yaml: read the plan, pick the next
  eligible change, run define → build → merge, and update status. Layer 2
  automation over the Layer 1 specify plan CLI.
license: MIT
argument-hint: "[--dry-run] [--loop]"
---

# Execute skill

Drive an initiative through `.specify/plan.yaml` by automating the
Layer 1 loop: `get next change` → `/spec:define` → `/spec:build` →
`/spec:merge` (or `/spec:drop`) → `specify plan transition`.

> **Scope note.** This revision (RFC-2 L2.E + L2.F) ships the skill
> scaffold, the `--dry-run` path, and the **supervised single-change**
> path (one change end to end, then stop). Self-heal on startup lands
> in L2.G; `--loop` in L2.H; `sources` / `affects` execution wiring in
> L2.I. Self-heal is stubbed here — the algorithm names the step and
> where it lives, and the fleshed-out implementation lands in L2.G.

## Overview

Specify at runtime is a three-layer stack:

1. **Phase skills** (`/spec:define`, `/spec:build`, `/spec:merge`,
   and `/spec:drop`) — the define-build-merge loop that operates on a
   single change.
2. **Plan CLI** (`specify plan {validate, next, status, create,
   amend, transition, archive, lock}`) — the library-backed verbs
   that read and write `.specify/plan.yaml`. Both humans (Layer 1)
   and this skill (Layer 2) drive the loop through these verbs; no
   other code path writes the plan file.
3. **Driver skill** (`/spec:execute`, this one) — the Layer 2
   automation that reads `plan.yaml`, picks the next entry, invokes
   the phase sequence, and records outcomes.

The on-disk contracts the driver depends on are the same files humans
read in Layer 1 — `/spec:execute` introduces no new storage of its own:

| File | Owner | Role |
|---|---|---|
| `.specify/plan.yaml` | library (`Plan::{create, amend, transition, archive}`) | Ordered change list with per-entry status. Driver reads via `specify plan next`/`status`; writes only via `specify plan transition`. |
| `.specify/changes/<name>/.metadata.yaml` | library (`ChangeMetadata` + `specify change phase-outcome`) | Change lifecycle status **and** the phase's `outcome` field. Phases stamp this; the driver reads it on phase return. |
| `.specify/changes/<name>/journal.yaml` | library (`Journal::append` + `specify change journal-append`) | Append-only audit log of `question` / `failure` / `recovery` entries. Never consumed as a signalling channel — `.metadata.yaml:outcome` is the only state the driver reads. |
| `.specify/plan.lock` | library (`PlanLockStamp`) | Advisory PID stamp held by the running driver. Prevents two `/spec:execute` invocations racing on the same plan. |

See [RFC-2 §"Layer 2: Automated Execution"](../../../../rfcs/rfc-2-execution.md)
for the full design, including the [Phase Outcome Contract](../../../../rfcs/rfc-2-execution.md),
[Plan Mutation and Crash Safety](../../../../rfcs/rfc-2-execution.md),
and [Driver Concurrency](../../../../rfcs/rfc-2-execution.md) sections.

## Invariants (RFC-2 §"Invariants")

These invariants constrain this skill's behaviour. They are
reproduced here verbatim from the RFC so the contract is auditable
without leaving the skill file.

| Invariant | Enforced by |
|---|---|
| Driver contracts with phases, not briefs | `/spec:execute` only invokes `/spec:define`, `/spec:build`, `/spec:merge` |
| Phases own verify-repair loops | Phase skills exhaust their repair budget before returning |
| Exactly one of `success`/`failure`/`deferred` per phase | Phase writes `outcome` into `.metadata.yaml` before returning (see [§Phase Outcome Contract](../../../../rfcs/rfc-2-execution.md)) |
| Change *entries* written only via `Plan::create` / `Plan::amend` | Phases and humans both run `specify plan create` / `specify plan amend` |
| Change *status* updates written only via `Plan::transition` | `/spec:execute` (Layer 2) or humans (Layer 1) run `specify plan transition` |
| Single `in-progress` at a time | `plan next` / `plan validate` |
| Single `/spec:execute` driver at a time | `.specify/plan.lock` advisory lock (see [§Driver Concurrency](../../../../rfcs/rfc-2-execution.md)) |

## Invocation

```text
/spec:execute              # supervised mode: run one change, stop (L2.F)
/spec:execute --dry-run    # preview next change + progress; no writes
/spec:execute --loop       # run until no eligible change (L2.H)
```

The plan path is fixed at `.specify/plan.yaml`; multi-plan support
is a future capability.

## Driver lock

`/spec:execute` takes the `.specify/plan.lock` PID stamp at the
start of every run — **including `--dry-run`** — and releases it on
normal exit. The stamp is managed by three dedicated CLI verbs:

```bash
specify plan lock acquire --pid <agent-session-pid>
specify plan lock status
specify plan lock release --pid <agent-session-pid>
```

Notes on the protocol:

- The stamp is a **PID file with liveness check**, not an
  `flock(2)`. Short-lived CLI invocations cannot hold an advisory
  file lock across agent-side work, so the lock is represented as a
  persistent marker that outlives the `specify` processes writing
  it. Self-heal in L2.G reclaims stale stamps (dead PID, malformed
  contents) before entering the loop proper.
- `--pid` defaults to `std::process::id()` of the `specify` binary.
  `/spec:execute` should pass a **stable agent-session PID** on every
  invocation so `release` can authenticate the holder. A missing
  `--pid` still works for a single acquire/release cycle within one
  `specify` process, but that isn't the shape this skill takes.
- Another live holder surfaces as `Error::DriverBusy { pid }` (exit
  code `1`); this skill reports the conflict and stops without
  touching the plan.
- The long-lived in-process `PlanLockGuard` primitive (with a real
  `flock`) remains available for any future native driver that
  keeps a Rust process alive for the full run.

## Step-by-step behaviour

### `--dry-run`

```text
1. Resolve the project directory (walk upward from CWD looking for
   .specify/project.yaml). Exit non-zero with a clear diagnostic if
   no Specify project is found.

2. Acquire the driver lock:
     specify plan lock acquire --pid <agent-session-pid>
   On Error::DriverBusy, report which PID holds the lock and exit 1
   without touching the plan.

3. Self-heal (L2.G — NOT YET IMPLEMENTED):
   In a full run, before any `get next change`, scan plan.yaml for
   in-progress entries and read their .metadata.yaml:outcome to
   resolve terminal outcomes left by a prior interrupted driver.
   For --dry-run in this Change, self-heal is intentionally skipped;
   dry-run reports the plan as-is. Full --dry-run under L2.G will
   still run self-heal under the lock (it is read-only with respect
   to the loop but writes plan transitions when it reclaims).

4. Query the plan:
     specify plan next --format json
     specify plan status --format json
   Interpret the `next` response shape:
     - `next: "<name>"` — entry to report.
     - `reason: "in-progress"` — name the active change and exit.
     - `reason: "all-done"` — initiative complete; print summary and exit.
     - `reason: "stuck"` — no eligible entry; print pending/blocked
       counts and the blocking dependencies.

5. Render the dry-run output (see §Output format). Every line starts
   with `[dry-run] ` so the operator cannot mistake a preview for
   a real run.

6. Release the driver lock:
     specify plan lock release --pid <agent-session-pid>
   Always run release on exit — success, dry-run stop, or
   DriverBusy-style early exit where the acquire succeeded.
```

### Supervised single-change run (no `--loop`)

Supervised mode runs exactly one change to a terminal plan status
(`done` / `failed` / `blocked`) and stops. The driver never iterates
to a second change in this mode — the `--loop` extension lands in
L2.H and layers on top of this same per-change algorithm.

The algorithm below is normative. Follow it step by step. Every
shell-out is to the Layer 1 `specify` CLI; this skill writes nothing
to `.specify/plan.yaml`, `.metadata.yaml`, or `journal.yaml` directly.

```text
1. Resolve the project directory (walk upward from CWD looking for
   .specify/project.yaml). Exit non-zero with a clear diagnostic if
   no Specify project is found.

2. Acquire the driver lock:
     specify plan lock acquire --pid <agent-session-pid>
   On Error::DriverBusy, report which PID holds the lock and exit 1
   without touching the plan.

3. Self-heal (L2.G — STUBBED here; fleshed out in L2.G):
   Scan .specify/plan.yaml for entries with status: in-progress. For
   each such entry, read .specify/changes/<name>/.metadata.yaml and
   inspect `.outcome.outcome`:
     - success  → apply `specify plan transition <name> done`
     - failure  → apply `specify plan transition <name> failed
                        --reason "<outcome.summary>"` (after running
                        /spec:drop <name> if the change directory is
                        still active — see steps 10/11 below)
     - deferred → same shape as failure but with `blocked`
   If the outcome is missing or malformed and a change directory is
   still active, resumption-within-a-change applies (RFC-2 §"Context
   Threading → Resumption Within a Change"); otherwise halt with a
   non-zero exit and leave the entry as in-progress for human triage.
   For L2.F, this step is a stub: the driver assumes plan.yaml has no
   stale in-progress entries. L2.G will supply the full implementation
   along with its own fixtures.

4. Pick the next change:
     specify plan next --format json
   Interpret the JSON:
     - `next != null`                → continue to step 5 with this
                                        name. Capture the entry's
                                        `description` and `affects`
                                        list for use in step 6.
     - `reason == "in-progress"`     → an active entry exists that
                                        self-heal did not resolve
                                        (either this is L2.F pre-G
                                        and the stub skipped it, or
                                        L2.G classified the state as
                                        ambiguous). Emit a diagnostic
                                        naming the entry, release the
                                        lock, exit non-zero.
     - `reason == "all-done"`        → emit the terminal "initiative
                                        complete" summary, release the
                                        lock, exit 0.
     - `reason == "stuck"`           → emit the terminal "stuck"
                                        summary (pending / blocked /
                                        failed buckets), release the
                                        lock, exit 0. `--loop` in L2.H
                                        treats this the same way.

5. Transition the selected entry:
     specify plan transition <name> in-progress
   This is the first plan write the driver performs. It must happen
   BEFORE /spec:define creates the change directory (RFC-2 §"Plan
   Mutation and Crash Safety"): between this step and step 6 the
   plan briefly shows an in-progress entry with no matching change
   directory, which `specify plan validate` tolerates as a warning.

6. Invoke /spec:define <name>. Pass the plan entry's `description`
   and `affects` list as additional context when present. Source-path
   resolution (the `sources` list → resolved paths for /spec:extract)
   is deferred to L2.I; for L2.F the define phase receives only the
   change name (and, optionally, the description / affects hints for
   its own use).

   When /spec:define returns, read the phase outcome per step 9.

7. If define returned success: invoke /spec:build <name>. On return,
   read the phase outcome per step 9.

8. If build returned success: invoke /spec:merge <name>. On return,
   read the phase outcome per step 9.

9. Read the phase outcome.

   Run:
     specify change status <name> --format json
   and take `.outcome.outcome` (the field lives at
   .specify/changes/<name>/.metadata.yaml:outcome.outcome). Classify:

     - success  → continue to the next phase in the sequence; after
                  /spec:merge succeeds, go to step 10.
     - failure  → go to step 11 (drop path).
     - deferred → go to step 12 (defer path).
     - missing / malformed / contradicts lifecycle status →
                  Treat as deferred with a synthetic summary:
                  "phase outcome missing after <phase>; driver
                  stopping for triage." Go to step 12. (This matches
                  the RFC-2 §"Phase Outcome Contract" fallback:
                  the driver never speculates about a missing outcome.)

10. Success wrap-up.
      specify plan transition <name> done
    Emit the success transcript (see §Output format). Go to step 13.

11. Failure drop path.
    a. Capture `outcome.summary` (and, if present, `outcome.context`)
       from the phase that failed.
    b. Run:
         /spec:drop <name> --reason "<outcome.summary>"
       This is the existing drop skill — it archives partial artifacts
       and flips the change lifecycle to `dropped`. It does NOT touch
       plan.yaml.
    c. Run:
         specify plan transition <name> failed --reason "<outcome.summary>"
       The `--reason` value is copied VERBATIM from the phase's
       `outcome.summary`. Do not paraphrase, truncate, or re-render.
    d. Emit the failure transcript. Go to step 13.

12. Deferred path (same shape as failure, with `blocked` instead of
    `failed`).
    a. Capture `outcome.summary` (and optional `outcome.context`).
    b. Run:
         /spec:drop <name> --reason "<outcome.summary>"
    c. Run:
         specify plan transition <name> blocked --reason "<outcome.summary>"
       `--reason` is copied verbatim, as in step 11c.
    d. Emit the deferred transcript. Go to step 13.

13. Release the driver lock:
      specify plan lock release --pid <agent-session-pid>
    Run this on EVERY exit path — success, failure, deferral, stop-
    for-triage (step 4 in-progress branch), or any uncaught error
    after step 2. The release step is unconditional; think of it as
    the trailing edge of a `try` / `finally` wrapping steps 3–12.
    Exit with code 0 for success/failure/deferred outcomes (the
    change reached a terminal plan status as designed), non-zero for
    step-4 in-progress stops and step-9 synthetic-deferred cases
    where human triage is required.
```

#### Subtleties

- **`/spec:execute` writes only plan transitions.** Every write this
  skill performs against `.specify/plan.yaml` goes through `specify
  plan transition`. It never writes `outcome` to `.metadata.yaml`
  (the phase does that, via `specify change phase-outcome`) and never
  appends to `journal.yaml` (the phase does that, via `specify change
  journal-append`). RFC-2 L2.G's self-heal step will append a single
  `type: recovery` entry when it reclaims an in-progress entry — that
  is the one and only case where the driver writes the journal, and
  it lives in L2.G not here.

- **Summary is copied verbatim into `status-reason`.** The string
  passed to `specify plan transition … --reason "…"` in steps 11c and
  12c is byte-identical to `outcome.summary` stamped by the phase.
  The fixtures under `fixtures/single-change/` pin this: every
  `plan.yaml.after` carries `status-reason: "<exact summary from the
  metadata file>"`. Do not paraphrase, truncate, or reformat.

- **Journal entries from the phase are preserved.** Whatever
  `type: question` / `type: failure` entries the phase wrote during
  its run stay on disk unchanged. The driver does not rewrite, merge,
  or summarise them. Humans reading the journal after a failure or
  deferral see the full trail the phase recorded, not a driver-
  authored post-hoc rollup.

- **Release the lock on every exit path.** Every branch of the
  algorithm — success, failure, deferred, stop-for-triage, unhandled
  error — MUST run `specify plan lock release` before returning
  control to the caller. Treat the release as the invariant trailing
  edge of the run. Stale stamps can be reclaimed by a later run, but
  only after a visible-to-the-operator failure, so leaving one behind
  is an observable defect.

- **Single `in-progress` at a time.** The driver never has more than
  one plan entry in `in-progress` at any point in time. Step 5 is
  the only place the driver enters that state; steps 10/11/12 are
  the only places the driver leaves it. Self-heal (L2.G) is the only
  other step that mutates plan status, and it only does so to resolve
  a pre-existing `in-progress` left by a prior crashed run.

### Self-heal (L2.G) and `--loop` (L2.H)

Out of scope for this revision. Step 3 of the supervised algorithm
above names the self-heal contract precisely; L2.G supplies the
implementation plus its own fixtures. `--loop` (L2.H) reuses steps
3–13 unchanged and wraps them in an outer iteration whose terminating
conditions are the step-4 `all-done` / `stuck` branches.

## Output format

### `--dry-run`

`--dry-run` emits a compact snapshot of the plan's current state and
the change the driver *would* run next. The first line names the
plan; the progress line has exactly six status counters in a fixed
order so downstream parsers see a stable shape; the next-change
block names the chosen entry along with its `sources` and `affects`
hints (omitting either when empty); the last line is an explicit
"nothing happened" claim.

```text
[dry-run] /spec:execute — <plan-name>

Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

Next: <name> (sources: [<sources>], affects: [<affects>])

No changes written.
```

Variants the skill picks based on `specify plan next`:

- `reason: "in-progress"` — replace the `Next:` line with:
  ```text
  Active: <name> (driver would resume/adopt this entry)
  ```
- `reason: "all-done"` — replace with:
  ```text
  Initiative complete — no eligible changes remain.
  ```
- `reason: "stuck"` — replace with:
  ```text
  Stuck — no eligible changes. Pending: [<names>]. Blocked: [<names>]. Failed: [<names>].
  ```

The canonical shape for a happy-path dry-run is pinned by the
snapshot fixture at [`fixtures/dry-run/`](fixtures/dry-run/): a
seed `plan.yaml` paired with the exact rendered `expected-output.md`.
Keep the two in sync when editing either side.

### Supervised single-change run

For a full supervised run, the transcript has three variants
(success / failure / deferred). Each is pinned by a behavioural
fixture under [`fixtures/single-change/`](fixtures/single-change/).

#### Success

```text
## /spec:execute — <plan-name>

### Initiative: <plan-name>
Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

---

### Processing: <name> (sources: [<sources>], affects: [<affects>])

Step 1/3: define
  - extract sub-step (via /spec:extract)
      Source: <path>
      Artifacts: specs/<name>/spec.md, design.md ✓
  Artifacts: proposal.md, specs, design.md, tasks.md ✓

Step 2/3: build
  Tasks: N/M complete ✓

Step 3/3: merge
  Baseline updated: .specify/specs/<name>/spec.md ✓
  Status: done
```

The `(sources: [...], affects: [...])` suffix on the Processing
line is rendered only for fields that are non-empty on the plan
entry; greenfield entries with neither become
`### Processing: <name> (greenfield)`. The extract sub-step block
inside `Step 1/3: define` is elided when the entry has no `sources`.

#### Failure

```text
## /spec:execute — <plan-name>

### Initiative: <plan-name>
Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

---

### Processing: <name> (sources: [<sources>], affects: [<affects>])

Step 1/3: define
  Artifacts: proposal.md, specs, design.md, tasks.md ✓

Step 2/3: build
  ✗ Build failed — change dropped, plan entry transitioned to failed

  Summary: <outcome.summary verbatim>
  Journal: .specify/changes/<name>/journal.yaml
  Action needed: Fix the underlying error, then retry via
    specify plan transition <name> pending
  Status: failed
```

The phase that fails is whichever returned `outcome: failure`; the
step header (`Step 1/3`, `Step 2/3`, or `Step 3/3`) names that phase.
The `Summary:` line is the phase's `outcome.summary` string, copied
byte-for-byte from `.metadata.yaml`.

#### Deferred

```text
## /spec:execute — <plan-name>

### Initiative: <plan-name>
Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

---

### Processing: <name> (greenfield)

Step 1/3: define
  ⚠ Question recorded — change deferred to blocked

  Question: <outcome.summary verbatim>
  Journal: .specify/changes/<name>/journal.yaml
  Action needed: Update the plan description (specify plan amend …) with the missing
    scope, then unflag (blocked → pending) to retry.
  Status: blocked
```

The `⚠ Question recorded — change deferred to blocked` line is the
canonical deferred banner from RFC-2 §"Output and Observability"; do
not reword it. `Question:` carries the phase's `outcome.summary`
verbatim. The `Action needed:` text is advisory and assumes a
description-level fix; real phases may recommend a different remedy
via the journal's `context` field, but the transcript itself stays on
this shape.

## What this skill does NOT do (in this Change)

| Surface | Status |
|---|---|
| Write `.specify/plan.yaml` *entries* (`create` / `amend`) | Never — those writes are the phases' concern (they shell out to `specify plan create` / `specify plan amend` mid-run). |
| Write `.specify/plan.yaml` *status* (`transition`) | Only via `specify plan transition`, at exactly three points in a supervised run: `pending → in-progress` before step 6, and the terminal `in-progress → {done, failed, blocked}` in steps 10/11/12. |
| Write `.specify/changes/<name>/.metadata.yaml` (including the `outcome` field) | Never — that is the phase skills' concern via `specify change phase-outcome`. |
| Write `.specify/changes/<name>/journal.yaml` | Never in L2.F. Phases append `type: question` / `type: failure` entries via `specify change journal-append`; L2.G's self-heal appends a single `type: recovery` entry when it reclaims. |
| Invoke `/spec:define`, `/spec:build`, `/spec:merge`, or `/spec:drop` | Never in `--dry-run`; in supervised mode, exactly as the algorithm above prescribes (define → build → merge on success paths; plus `/spec:drop` on failure / deferred). |
| Run self-heal on `in-progress` entries | Step 3 of the supervised algorithm names the contract; implementation is stubbed in this Change, fleshed out in L2.G. |
| Loop across changes | `--loop` lands in L2.H. |
| Resolve `sources` keys to paths / URLs and hand them to define | Deferred to L2.I. In L2.F, `/spec:define` is invoked with the change name only; the plan entry's `description` and `affects` list are passed along when present, but `sources` resolution is not. |

The state the skill mutates in this Change is:

1. The driver lock stamp at `.specify/plan.lock` (written on acquire,
   removed on release by the CLI — not by the skill directly).
2. The plan entry's `status` field via `specify plan transition` at
   the three points named above.

No other on-disk state is written by `/spec:execute` itself.

## Guardrails

- Never hand-edit `.specify/plan.yaml`, `.specify/changes/<name>/.metadata.yaml`,
  or `.specify/changes/<name>/journal.yaml`. Route every write through
  the CLI verbs above — the single-writer invariant in RFC-2
  §"Plan Mutation and Crash Safety" depends on it.
- Never skip the lock-release step. If the skill exits early after a
  successful acquire (error in subsequent steps, user abort), run
  `specify plan lock release --pid <agent-session-pid>` on the way
  out. Stale stamps can be reclaimed by a later run, but only after
  a visible-to-the-operator failure.
- Treat an unexpected `specify plan next` response shape (missing
  keys, unknown `reason`) as a hard failure: print the raw JSON,
  release the lock, and exit non-zero. Do not speculate.
- For `--dry-run` specifically: the skill MUST NOT invoke any phase
  skill, MUST NOT shell out to `specify plan transition`, and MUST
  prefix every line of its rendered output with `[dry-run] ` in the
  first-line banner (the progress / next blocks do not need a
  per-line prefix — the banner is enough).
- For the supervised single-change run: the string passed to
  `specify plan transition <name> {failed,blocked} --reason "…"` is
  always `outcome.summary` from the phase's `.metadata.yaml`, copied
  byte-for-byte. Never paraphrase, truncate, or add a prefix. The
  fixtures under `fixtures/single-change/` assert this equality; a
  drift between `metadata-after-*.yaml:outcome.summary` and
  `plan.yaml.after:status-reason` is a regression.
- Phase outcome missing or malformed after a phase returns means the
  phase crashed or skipped its `specify change phase-outcome` call.
  Treat as `deferred` with a synthetic summary
  (`"phase outcome missing after <phase>; driver stopping for triage."`)
  per RFC-2 §"Phase Outcome Contract" — do not speculate about which
  of success / failure was really intended.

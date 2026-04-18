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

> **Scope note.** This revision (RFC-2 L2.E + L2.F + L2.G + L2.H)
> ships the skill scaffold, the `--dry-run` path, the **supervised
> single-change** path (one change end to end, then stop), the
> **self-heal on startup** step that runs before every `get next
> change`, and the **`--loop`** extension plus terminal summary and
> SIGINT / SIGTERM handling. `sources` / `affects` execution wiring
> lands in L2.I.

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
/spec:execute --loop       # run until no eligible change remains (L2.H)
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
  it. `specify plan lock acquire` reclaims a stale stamp (dead PID,
  malformed contents) itself before the driver enters the self-heal
  step; nothing in this skill hand-rolls that check.
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

3. Self-heal (report-only in `--dry-run`):
   Run the self-heal scan described in §Self-heal on startup
   BEFORE the plan query in step 4, but in REPORT-ONLY mode. For
   every `in-progress` entry the scan classifies under the algorithm
   in §Self-heal on startup, print the transition the writing path
   WOULD take, using the "Would transition" wording pinned in
   §Self-heal on startup → §Dry-run variant. Do NOT call
   `specify plan transition`, `specify change journal-append`, or
   `/spec:drop`. The `--dry-run` contract ("no writes to plan.yaml,
   .metadata.yaml, or journal.yaml") covers this step as well as
   the per-change define/build/merge loop — dry-run is read-only
   end to end. If the scan would halt (ambiguity case), print the
   same halt diagnostic, release the lock, and exit non-zero
   WITHOUT reaching step 4.

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

3. Self-heal:
   Run the self-heal algorithm described in §Self-heal on startup.
   Self-heal either (a) leaves plan.yaml untouched (no in-progress
   entries to reclaim), (b) resolves each in-progress entry to its
   terminal plan status by reading the phase outcome on disk, (c)
   signals "resume mid-change" for an entry whose phase was in-flight
   at crash time, or (d) halts for human triage on ambiguity. Cases
   (a) and (b) fall through to step 4. Case (c) skips step 4 and
   picks up at the appropriate phase (step 6, 7, or 8) for the named
   entry. Case (d) releases the lock and exits non-zero without
   reaching step 4.

4. Pick the next change:
     specify plan next --format json
   Interpret the JSON:
     - `next != null`                → continue to step 5 with this
                                        name. Capture the entry's
                                        `description` and `affects`
                                        list for use in step 6.
     - `reason == "in-progress"`     → an active entry exists that
                                        self-heal did not resolve
                                        (self-heal classified the
                                        state as ambiguous and halted
                                        — this branch should never
                                        fire in practice because the
                                        halt in step 3 exits before
                                        reaching step 4; it remains
                                        here as a defence in depth).
                                        Emit a diagnostic naming the
                                        entry, release the lock, exit
                                        non-zero.
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

- **`/spec:execute` writes only plan transitions and recovery journal
  entries.** Every write this skill performs against `.specify/plan.yaml`
  goes through `specify plan transition`. It never writes `outcome` to
  `.metadata.yaml` (the phase does that, via `specify change
  phase-outcome`). The sole case in which the driver appends to
  `journal.yaml` is the self-heal step (§Self-heal on startup), which
  emits exactly one `type: recovery` entry per reclaimed or resumed
  in-progress entry via `specify change journal-append`. The define /
  build / merge phases own `type: question` and `type: failure`
  entries; the driver never touches those.

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
  the only places the driver leaves it. Self-heal (§Self-heal on
  startup) is the only other step that mutates plan status, and it
  only does so to resolve a pre-existing `in-progress` left by a
  prior crashed run.

### Self-heal on startup

Self-heal is the driver's reconciliation pass. It runs **once per
`/spec:execute` invocation**, under the driver lock, immediately
after the lock is acquired and before `specify plan next`. Its job
is to make the plan agree with what actually finished on disk before
the previous driver crashed — not to do any new work.

Two invariants frame the whole step. First, `.metadata.yaml:outcome`
is the single authoritative signal: the driver never consults
`journal.yaml`, tempfiles, or stderr transcripts to decide what
happened. Second, nothing in self-heal speculates. Every ambiguity
(missing outcome with no change dir, outcome field that contradicts
`LifecycleStatus`, two archives with equal timestamps, …) halts the
driver with a non-zero exit so a human can triage. A speculative
transition could silently mark a failed change as `done` and that
failure mode is strictly worse than "one extra triage per N runs".

#### Algorithm

For every plan entry `E` whose `status` is `in-progress`, in the
order they appear in `plan.yaml` (the loop tolerates more than one
even though `specify plan next` would flag that as a validation
warning):

1. **Locate the latest `.metadata.yaml` for `E.name`.**
   - First check `.specify/changes/<name>/.metadata.yaml` — the
     active change directory. If present, that is the file to read.
   - If absent, inspect `.specify/archive/`. Archive directory
     names end with `-<name>` (the `YYYY-MM-DD-<name>` convention
     from `specify change archive` — see RFC-1 §`metadata.rs`).
     Multiple matches are legal: a change that failed and was later
     retried leaves one archive per attempt. Pick the most recent by
     the `created-at` field inside its `.metadata.yaml` (falling back
     to `defined-at`, then the directory's `YYYY-MM-DD` prefix as a
     final tie-break). Prefer:
     ```bash
     specify change status <name> --format json
     ```
     when the CLI surfaces the archive lookup; otherwise a shell
     `ls -t .specify/archive/ | grep -- '-<name>$' | head -n1` gives
     the same answer from directory mtimes.
   - If nothing is found anywhere, the plan entry is `in-progress`
     but no change has ever been created — typically because the
     prior driver crashed between `specify plan transition pending →
     in-progress` (step 5) and `/spec:define` (step 6). Treat this as
     mid-change resume with `LifecycleStatus = None` — jump to step
     3 below.

2. **Read `.metadata.yaml.outcome` and act on it.**
   - `outcome.outcome == success` and `outcome.phase == merge` →
     the merge finished but the driver crashed before the terminal
     plan transition. Run:
     ```bash
     specify plan transition <name> done
     ```
     No `status-reason` on success.
   - `outcome.outcome == success` and `outcome.phase ∈ {define, build}`
     → the phase finished but the driver crashed before launching
     the next phase. This is **not** a terminal state. Do NOT
     transition the plan. Fall through to step 3 with
     `LifecycleStatus ∈ {defined, complete}` to resume the next
     phase in the sequence.
   - `outcome.outcome == failure` → run `/spec:drop <name>
     --reason "<outcome.summary>"` (same drop skill steps 11a/b
     invoke for a live failure — it is idempotent against an
     already-dropped change). Then:
     ```bash
     specify plan transition <name> failed --reason "<outcome.summary>"
     ```
     The `--reason` string is copied byte-for-byte from
     `outcome.summary`; never paraphrase or truncate.
   - `outcome.outcome == deferred` → same shape as failure but with
     `blocked`:
     ```bash
     /spec:drop <name> --reason "<outcome.summary>"
     specify plan transition <name> blocked --reason "<outcome.summary>"
     ```
   - The `outcome` field is **absent** and `LifecycleStatus` is
     non-terminal (`defining`, `defined`, `building`, `complete`) →
     no terminal outcome was ever stamped; the prior phase was
     in-flight at crash time. Fall through to step 3 with the
     on-disk `LifecycleStatus`. This is the explicit "active change
     dir with no terminal outcome yet" branch from RFC-2 §"Plan
     Mutation and Crash Safety".
   - The `outcome` field is malformed (unknown enum variant, missing
     `phase`, missing `summary`, …) or its `phase` contradicts
     `LifecycleStatus` (for example `phase: merge` with `status:
     defining`, or `outcome: success` with `status: dropped`,
     anything `LifecycleStatus::can_transition_to` would reject)
     → **halt**. Emit the diagnostic line below with exit code 1.
     Leave the plan entry as `in-progress`. Do not append a recovery
     journal entry. Do not drop the change. Humans triage.

3. **Mid-change resume** (no terminal outcome yet). This branch is
   reached either when step 1 found no metadata at all, when step 2
   saw `outcome.outcome == success` on `define` / `build`, or when
   step 2 saw no `outcome` field alongside a non-terminal
   `LifecycleStatus`. Read `LifecycleStatus` from
   `.metadata.yaml.status` (missing metadata counts as "None") and
   apply the resumption table from RFC-2 §"Context Threading →
   Resumption Within a Change":
   - `None` → invoke `/spec:define <name>` from scratch (step 6 of
     the supervised run, skipping step 5 because `plan.yaml` already
     has the entry `in-progress`).
   - `defining` → resume / restart `/spec:define <name>`.
   - `defined` → invoke `/spec:build <name>` (step 7).
   - `building` → resume `/spec:build <name>`.
   - `complete` → invoke `/spec:merge <name>` (step 8).
   - `merged` or `dropped` → contradiction. The change is in a
     terminal lifecycle state but the plan still carries `in-progress`.
     **Halt** — same semantics as the step-2 ambiguity branch.
   Resumption does NOT write a plan transition — the entry stays
   `in-progress` while the resumed phase sequence completes; the
   normal step-9 phase-outcome read wraps up the run.

4. **Append one `type: recovery` entry to `journal.yaml`** for every
   entry self-heal actually resolved or resumed (not for halts):
   ```bash
   specify change journal-append <name> <phase> recovery \
       --summary "Self-heal on startup: <action>" \
       --context "before=<plan-status>/<lifecycle-status>, after=<resolved-status-or-phase>"
   ```
   where `<phase>` is the phase the recovery relates to (the
   `outcome.phase` for terminal cases; the phase about to run for
   mid-change resume). Example `<action>` strings:
   - `"applied terminal transition done after finding success outcome on merge"`
   - `"applied terminal transition failed after finding failure outcome on build"`
   - `"applied terminal transition blocked after finding deferred outcome on define"`
   - `"resumed mid-change build phase (LifecycleStatus=defined)"`

5. **Lock scope.** Self-heal runs **inside** the driver lock already
   acquired at step 2 of the outer run. There is no second acquire
   and no inner release: the whole reconciliation pass is part of
   the same critical section that wraps the per-change loop. Two
   `/spec:execute` invocations started at the same time cannot both
   enter self-heal — the second one fails at `specify plan lock
   acquire` with `Error::DriverBusy`.

#### Diagnostic output

Every self-heal action emits exactly one line to stdout so an
operator watching the driver start up can see what happened:

```text
Self-heal: no in-progress entries found.
Self-heal: <name> → done (merge success from prior run)
Self-heal: <name> → failed (build failure: "<outcome.summary verbatim>")
Self-heal: <name> → blocked (define deferred: "<outcome.summary verbatim>")
Self-heal: <name> — resuming <phase> (LifecycleStatus=<lifecycle>)
Self-heal halted: <name> has outcome=<outcome> phase=<phase> but LifecycleStatus=<lifecycle>. Manual triage required.
```

The halt variant is followed by `Exit 1`; the other variants fall
through to step 4 of the supervised run. Fixture-pinned examples of
each line live under `fixtures/self-heal/`.

### `--loop` (L2.H)

Out of scope for this revision. `--loop` (L2.H) reuses steps 3–13
unchanged and wraps them in an outer iteration whose terminating
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
| Write `.specify/changes/<name>/journal.yaml` | Only via `specify change journal-append <name> <phase> recovery …` inside the §Self-heal on startup step — exactly one entry per reclaimed or resumed in-progress entry. Phases own the `type: question` / `type: failure` entries and the driver never touches those. |
| Invoke `/spec:define`, `/spec:build`, `/spec:merge`, or `/spec:drop` | Never in `--dry-run`; in supervised mode, exactly as the algorithm above prescribes (define → build → merge on success paths; plus `/spec:drop` on failure / deferred, and on any self-heal reclaim of a `failure` or `deferred` outcome). |
| Run self-heal on `in-progress` entries | Yes — §Self-heal on startup is the full contract. Four/five fixtures under `fixtures/self-heal/` pin the clean / done / failed / ambiguous-halt / mid-change-resume paths. |
| Loop across changes | `--loop` lands in L2.H. |
| Resolve `sources` keys to paths / URLs and hand them to define | Deferred to L2.I. In L2.F, `/spec:define` is invoked with the change name only; the plan entry's `description` and `affects` list are passed along when present, but `sources` resolution is not. |

The state the skill mutates in this Change is:

1. The driver lock stamp at `.specify/plan.lock` (written on acquire,
   removed on release by the CLI — not by the skill directly).
2. The plan entry's `status` field via `specify plan transition` at
   the three points named in the supervised-run algorithm, plus any
   terminal transitions self-heal applies on startup.
3. A single `type: recovery` entry appended to
   `.specify/changes/<name>/journal.yaml` via `specify change
   journal-append` whenever self-heal resolves or resumes an
   in-progress entry.

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
- Self-heal applies the same verbatim-`summary` rule as steps 11c /
  12c: the string passed to `specify plan transition <name>
  {failed,blocked} --reason "…"` is copied byte-for-byte from the
  on-disk `outcome.summary`. The fixtures under `fixtures/self-heal/`
  assert this equality; drift is a regression.
- Self-heal never paraphrases ambiguity away. If `.metadata.yaml` has
  no `outcome`, an `outcome` with a `phase` that contradicts
  `LifecycleStatus`, or a `LifecycleStatus` that is terminal
  (`merged`, `dropped`) while `plan.yaml` still says `in-progress`,
  halt with exit code 1 and leave the plan entry as `in-progress`.
  A later run, after human triage, can re-enter self-heal safely.

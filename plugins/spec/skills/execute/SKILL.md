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

> **Scope note.** This Change (RFC-2 L2.E) ships the skill scaffold
> and the `--dry-run` path **only**. Full-run behaviour (phase
> invocation, outcome interpretation, plan transitions) lands in
> L2.F; self-heal on startup in L2.G; `--loop` in L2.H. Invoking
> the skill without `--dry-run` today surfaces a clear "not-yet-
> implemented" diagnostic and stops.

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

### `--dry-run` (this Change)

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

### Full run (L2.F), self-heal (L2.G), and `--loop` (L2.H)

Out of scope for this Change. The RFC-2 §"Core Loop" normative
pseudocode is reproduced below for context; the behavioural fixtures
that pin the outputs land with the Changes that implement each path.

```text
  1. Read plan.yaml
  2. Select next eligible change (all depends-on are done, status is pending)
  3. If none eligible → stop (report blocked/remaining counts)
  4. Transition plan entry: pending → in-progress
  5. Run phase sequence: /spec:define, then /spec:build, then /spec:merge.
     Each phase stamps `outcome` in .metadata.yaml before returning.
  6. On success: transition in-progress → done
  7. On failure: invoke /spec:drop, transition in-progress → failed,
                 copy outcome.summary → status-reason
  8. On deferred: invoke /spec:drop, transition in-progress → blocked,
                  copy outcome.summary → status-reason
  9. If --loop: continue from step 1; otherwise stop
```

## Output format

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

## What this skill does NOT do (in this Change)

| Surface | Status |
|---|---|
| Write `.specify/plan.yaml` | Never — only `specify plan {create, amend, transition, archive}` mutate the plan, and dry-run invokes none of them. |
| Write `.specify/changes/<name>/.metadata.yaml` | Never — that is the phase skills' concern via `specify change phase-outcome`. |
| Write `.specify/changes/<name>/journal.yaml` | Never — phases append via `specify change journal-append`. |
| Invoke `/spec:define`, `/spec:build`, `/spec:merge`, or `/spec:drop` | Never in `--dry-run`. Full-run phase invocation lands in L2.F. |
| Run self-heal on `in-progress` entries | Stubbed in this Change; real behaviour lands in L2.G. |
| Loop across changes | `--loop` lands in L2.H. |

The only state the skill mutates in this Change is the driver lock
stamp at `.specify/plan.lock`, which is written on acquire and
removed on release by the CLI — not by the skill directly.

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

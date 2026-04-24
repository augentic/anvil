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

Drive an initiative through `.specify/plan.yaml` by automating the Layer 1 loop: `get next change` → `/spec:define` → `/spec:build` → `/spec:merge` (or `/spec:drop`) → `specify plan transition`.

> **Status.** Layer 2 is fully landed as of RFC-2 closeout. RFC-3b extends the driver with multi-repo CWD routing (`project` field on plan entries), `plan next` field extensions (`project`, `description`, `sources` in JSON), workspace status checks, merge auto-commit in workspace clones, and self-heal under multi-repo. This skill ships the `--dry-run` preview, the supervised single-change run, the self-heal pass on startup, `--loop` mode with terminal summary and SIGINT / SIGTERM handling, and the `sources` execution wiring. `/spec:execute --loop` drives the [RFC-2 §"The Plan"](../docs/links.md#rfc-2-the-plan) example end-to-end against a plan authored by `/spec:plan` — see the [§Fixtures](#fixtures) table for the exit-gate meta-fixture.

## Overview

Specify at runtime is a three-layer stack:

1. **Phase skills** (`/spec:define`, `/spec:build`, `/spec:merge`, and `/spec:drop`) — the define-build-merge loop that operates on a single change.
2. **Plan CLI** (`specify plan {validate, next, status, create, amend, transition, archive, lock}`) — the library-backed verbs that read and write `.specify/plan.yaml`. Both humans (Layer 1) and this skill (Layer 2) drive the loop through these verbs; no other code path writes the plan file.
3. **Driver skill** (`/spec:execute`, this one) — the Layer 2 automation that reads `plan.yaml`, picks the next entry, invokes the phase sequence, and records outcomes.

The on-disk contracts the driver depends on are the same files humans read in Layer 1 — `/spec:execute` introduces no new storage of its own:

| File | Owner | Role |
|---|---|---|
| `.specify/plan.yaml` | library (`Plan::{create, amend, transition, archive}`) | Ordered change list with per-entry status. Driver reads via `specify plan next`/`status`; writes only via `specify plan transition`. |
| `.specify/changes/<name>/.metadata.yaml` | library (`ChangeMetadata` + `specify change phase-outcome`) | Change lifecycle status **and** the phase's `outcome` field. Phases stamp this; the driver reads it on phase return. |
| `.specify/changes/<name>/journal.yaml` | library (`Journal::append` + `specify change journal-append`) | Append-only audit log of `question` / `failure` / `recovery` entries. Never consumed as a signalling channel — `.metadata.yaml:outcome` is the only state the driver reads. |
| `.specify/plan.lock` | library (`PlanLockStamp`) | Advisory PID stamp held by the running driver. Prevents two `/spec:execute` invocations racing on the same plan. |

See [RFC-2 §"Layer 2: Automated Execution"](../docs/links.md#rfc-2-layer-2) for the full design, including the [Phase Outcome Contract](../docs/links.md#rfc-2-phase-outcome-contract), [Plan Mutation and Crash Safety](../docs/links.md#rfc-2-plan-mutation), and [Driver Concurrency](../docs/links.md#rfc-2-driver-concurrency) sections.

## Invariants (RFC-2 §"Invariants")

These invariants constrain this skill's behaviour. They are reproduced here verbatim from the RFC so the contract is auditable without leaving the skill file.

| Invariant | Enforced by |
|---|---|
| Driver contracts with phases, not briefs | `/spec:execute` only invokes `/spec:define`, `/spec:build`, `/spec:merge` |
| Phases own verify-repair loops | Phase skills exhaust their repair budget before returning |
| Exactly one of `success`/`failure`/`deferred` per phase | Phase writes `outcome` into `.metadata.yaml` before returning (see [§Phase Outcome Contract](../docs/links.md#rfc-2-phase-outcome-contract)) |
| Change *entries* written only via `Plan::create` / `Plan::amend` | Phases and humans both run `specify plan create` / `specify plan amend` |
| Change *status* updates written only via `Plan::transition` | `/spec:execute` (Layer 2) or humans (Layer 1) run `specify plan transition` |
| Single `in-progress` at a time | `plan next` / `plan validate` |
| Single `/spec:execute` driver at a time | `.specify/plan.lock` advisory lock (see [§Driver Concurrency](../docs/links.md#rfc-2-driver-concurrency)) |

## Invocation

```text
/spec:execute              # supervised mode: run one change, stop
/spec:execute --dry-run    # preview next change + progress; no writes
/spec:execute --loop       # run until no eligible change remains
```

The plan path is fixed at `.specify/plan.yaml`; multi-plan support is a future capability.

## Driver lock

`/spec:execute` takes the `.specify/plan.lock` PID stamp at the start of every run — **including `--dry-run`** — and releases it on normal exit. The stamp is managed by three dedicated CLI verbs:

```bash
specify plan lock acquire --pid <agent-session-pid>
specify plan lock status
specify plan lock release --pid <agent-session-pid>
```

Notes on the protocol:

- The stamp is a **PID file with liveness check**, not an `flock(2)`. Short-lived CLI invocations cannot hold an advisory file lock across agent-side work, so the lock is represented as a persistent marker that outlives the `specify` processes writing it. `specify plan lock acquire` reclaims a stale stamp (dead PID, malformed contents) itself before the driver enters the self-heal step; nothing in this skill hand-rolls that check.
- `--pid` defaults to `std::process::id()` of the `specify` binary. `/spec:execute` should pass a **stable agent-session PID** on every invocation so `release` can authenticate the holder.
- Another live holder surfaces as `Error::DriverBusy { pid }` (exit code `1`); this skill reports the conflict and stops without touching the plan.
- The long-lived in-process `PlanLockGuard` primitive (with a real `flock`) remains available for any future native driver that keeps a Rust process alive for the full run.

## Per-change algorithm

The per-change algorithm runs a single plan entry to a terminal status (`done` / `failed` / `blocked`) and is the core of every mode. The [§Modes](#modes) section below wires this into the three invocations (`--dry-run`, supervised, `--loop`) by describing only the deltas from this algorithm.

The algorithm is normative. Every shell-out is to the Layer 1 `specify` CLI; this skill writes nothing to `.specify/plan.yaml`, `.metadata.yaml`, or `journal.yaml` directly.

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
                                        `project`, `description`, and
                                        `sources` for use in steps 5a
                                        and 6 (see §Argument
                                        resolution).
     - `reason == "in-progress"`     → defence in depth: an active
                                        entry exists that self-heal
                                        did not resolve. In practice
                                        the halt in step 3 exits
                                        before reaching step 4. Emit
                                        a diagnostic naming the
                                        entry, release the lock, exit
                                        non-zero.
     - `reason == "all-done"`        → emit the terminal "initiative
                                        complete" summary, release
                                        the lock, exit 0.
     - `reason == "stuck"`           → emit the terminal "stuck"
                                        summary (pending / blocked /
                                        failed buckets), release the
                                        lock, exit 0. `--loop` treats
                                        this the same way.

5. Transition the selected entry:
     specify plan transition <name> in-progress
   This is the first plan write the driver performs. It must happen
   BEFORE /spec:define creates the change directory (RFC-2 §"Plan
   Mutation and Crash Safety"): between this step and step 6 the
   plan briefly shows an in-progress entry with no matching change
   directory, which `specify plan validate` tolerates as a
   warning.

5a. CWD routing (multi-repo only).
   Read `project` from the `specify plan next` response (step 4).
   If `project` is non-null:
     - Resolve the target directory from `registry.yaml`: relative-
       path `url` → resolved filesystem path; remote `url` →
       `.specify/workspace/<name>/`.
     - Check workspace freshness via `specify workspace status` for
       that slot. If `missing`, halt with a diagnostic pointing the
       operator at `specify workspace sync`. Release the lock and
       exit non-zero.
     - Save CWD (the initiating repo root).
     - Resolve every key in the entry's `sources` list to an absolute
       filesystem path anchored to the initiating repo root. Git URLs
       pass through unchanged.
     - `chdir` into the target project root.
     - Emit diagnostic: `Routing: <name> → <project> (<resolved-path>)`
   If `project` is null, skip this step entirely (pre-RFC-3b
   single-repo path).

6. Resolve the plan entry's `sources` into define arguments (see
   §Argument resolution below) and invoke:

     /spec:define <name> \
         [--source <key>=<path-or-url> [--source ...]]

   The `description` field on the plan entry carries scope and
   delta-targeting intent; define reads it off the plan directly and
   the driver does not re-plumb it through the command line.

   When /spec:define returns, read the phase outcome per step 9.

7. If define returned success: invoke /spec:build <name>. On return,
   read the phase outcome per step 9.

8. If build returned success: invoke /spec:merge <name>. On return,
   read the phase outcome per step 9.

9. Read the phase outcome.
   Run:
     specify change outcome <name> --format json
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
                  the RFC-2 §"Phase Outcome Contract" fallback: the
                  driver never speculates about a missing outcome.)

9a. CWD restore (multi-repo only).
   If the CWD routing step (5a) changed the working directory,
   restore CWD to the saved initiating repo root. This ensures
   `specify plan transition` (which reads `plan.yaml` in the
   initiating repo) runs from the correct directory. In `--loop`
   mode, the CWD routing and CWD restore steps bracket every
   iteration so that `specify plan next` always runs from
   the initiating repo root.

10. Success wrap-up.
      specify plan transition <name> done
    Emit the success transcript (see §Output format). Go to step 13.

11. Failure drop path.
    a. Capture `outcome.summary` (and, if present, `outcome.context`)
       from the phase that failed.
    b. Run:
         /spec:drop <name> --reason "<outcome.summary>"
       This is the existing drop skill — it archives partial
       artifacts and flips the change lifecycle to `dropped`. It does
       NOT touch plan.yaml.
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

### Argument resolution (`sources`)

Step 6 of the per-change algorithm turns the plan entry's `sources` field into command-line arguments for `/spec:define`. Scope and delta-targeting intent are carried in the entry's `description` field and inferred by the define skill; the driver does not forward them as separate flags.

- **`sources`** — a list of keys into the plan's top-level `sources` map. Each key resolves to either a local filesystem path or a git URL. The resolved values are handed to `/spec:define` as `--source <key>=<path-or-url>` tuples, preserving the key so define's brief pipeline can retain provenance when it hands the value to `/spec:extract` (via `git-cloner`) or an analogous plugin.

For every key in the plan entry's `sources` list, look it up in the plan's top-level `sources` map and classify the value:

1. **Key absent from the top-level map** — unresolved reference. The plan is internally inconsistent; this is an `Error::Config`-level halt. Emit a diagnostic naming the offending `(change, key)` pair, release the driver lock, exit non-zero. This should have been caught earlier by `specify plan validate` via the `unknown-source` diagnostic (RFC-2 Change L1.F), so reaching this branch means either the plan was not validated or it was edited out of band between validation and execution — either way, human triage.
2. **Value is a local filesystem path** (e.g. `/path/to/legacy`) — pass through as-is. The driver does NOT stat the path or verify it exists; `/spec:define` (and downstream `/spec:extract`) are responsible for surfacing a missing-path error with the right phase-level diagnostic.
3. **Value is a git URL** (e.g. `git@github.com:org/service.git` or `https://github.com/…`) — pass through as-is. The driver does NOT clone here. Cloning is `git-cloner`'s concern, invoked from inside `/spec:define`'s brief pipeline when a brief needs the source tree materialized. This keeps the clone cache under the phase's control and avoids duplicating the clone logic in the driver.

The path-vs-URL distinction is a content-level classification on the value string; neither `plan.schema.json` nor the plan library distinguishes them (both are validated as `type: string`). The driver emits the tuple as `--source <key>=<value>` unchanged — the classification matters only for the diagnostics rendered in the transcript.

Two authoring pins under [`fixtures/field-wiring/`](fixtures/field-wiring/) cover the two shapes — `sources-only/` (`/spec:define <name> --source monolith=/path/to/legacy`) and `description-driven/` (greenfield or description-inferred entries with no `--source` flags) — see the [§Fixtures](#fixtures) table for the invocation each one pins.

Under multi-repo routing (step 5a active), source paths from the plan's top-level `sources` map are resolved to **absolute filesystem paths** anchored to the initiating repo root before the CWD change. The resolved absolute paths are what gets passed to `/spec:define --source <key>=<absolute-path>`. This ensures source paths remain valid regardless of which project clone the driver has `chdir`'d into. Git URLs pass through unchanged.

### Subtleties

- **`/spec:execute` writes only plan transitions and recovery journal entries.** Every write this skill performs against `.specify/plan.yaml` goes through `specify plan transition`. It never writes `outcome` to `.metadata.yaml` (the phase does that, via `specify change phase-outcome`). The sole case in which the driver appends to `journal.yaml` is the self-heal step (§Self-heal on startup), which emits exactly one `type: recovery` entry per reclaimed or resumed in-progress entry via `specify change journal-append`. The define / build / merge phases own `type: question` and `type: failure` entries; the driver never touches those.

- **Summary is copied verbatim into `status-reason`.** The string passed to `specify plan transition … --reason "…"` in steps 11c and 12c is byte-identical to `outcome.summary` stamped by the phase. The fixtures under `fixtures/single-change/` pin this: every `plan.yaml.after` carries `status-reason: "<exact summary from the metadata file>"`. Do not paraphrase, truncate, or reformat.

- **Journal entries from the phase are preserved.** Whatever `type: question` / `type: failure` entries the phase wrote during its run stay on disk unchanged. The driver does not rewrite, merge, or summarise them. Humans reading the journal after a failure or deferral see the full trail the phase recorded, not a driver-authored post-hoc rollup.

- **Release the lock on every exit path.** Every branch of the algorithm — success, failure, deferred, stop-for-triage, unhandled error — MUST run `specify plan lock release` before returning control to the caller. Treat the release as the invariant trailing edge of the run.

- **Single `in-progress` at a time.** The driver never has more than one plan entry in `in-progress` at any point in time. Step 5 is the only place the driver enters that state; steps 10/11/12 are the only places the driver leaves it. Self-heal is the only other step that mutates plan status, and only to resolve a pre-existing `in-progress` left by a prior crashed run.

## Self-heal on startup

Self-heal is the driver's reconciliation pass. It runs **once per `/spec:execute` invocation**, under the driver lock, immediately after the lock is acquired and before `specify plan next`. Its job is to make the plan agree with what actually finished on disk before the previous driver crashed — not to do any new work.

Two invariants frame the whole step. First, `.metadata.yaml:outcome` is the single authoritative signal: the driver never consults `journal.yaml`, tempfiles, or stderr transcripts to decide what happened. Second, nothing in self-heal speculates. Every ambiguity (missing outcome with no change dir, outcome field that contradicts `LifecycleStatus`, two archives with equal timestamps, …) halts the driver with a non-zero exit so a human can triage. A speculative transition could silently mark a failed change as `done` and that failure mode is strictly worse than "one extra triage per N runs".

### Algorithm

For every plan entry `E` whose `status` is `in-progress`, in the order they appear in `plan.yaml` (the loop tolerates more than one even though `specify plan next` would flag that as a validation warning):

1. **Locate the latest `.metadata.yaml` for `E.name`.**
   - First check `.specify/changes/<name>/.metadata.yaml` — the active change directory. If present, that is the file to read.
   - If absent, inspect `.specify/archive/`. Archive directory names end with `-<name>` (the `YYYY-MM-DD-<name>` convention from `specify change archive` — see RFC-1 §`metadata.rs`). Multiple matches are legal: a change that failed and was later retried leaves one archive per attempt. Pick the most recent by `created-at` inside its `.metadata.yaml` (falling back to `defined-at`, then the directory's `YYYY-MM-DD` prefix); the outcome itself is surfaced by `specify change outcome <name> --format json` once the active directory is back in place.
   - If nothing is found anywhere, the plan entry is `in-progress` but no change has ever been created — typically because the prior driver crashed between `specify plan transition pending → in-progress` (step 5) and `/spec:define` (step 6). Treat this as mid-change resume with `LifecycleStatus = None` — jump to step 3 below.

**Multi-repo self-heal (RFC-3b).** For each `in-progress` entry `E` in `plan.yaml`, self-heal reads `E.project` from the plan entry. If non-null:
1. Resolve the target project directory from `registry.yaml` (same resolution as step 5a of the per-change algorithm).
2. Check workspace freshness for that slot. If `missing`, halt — same semantics as the main loop.
3. Look for `.specify/changes/<E.name>/.metadata.yaml` under the resolved project root instead of the initiating repo root. The classification logic (step 2 of self-heal) and recovery journal append (step 4) are unchanged.
4. Restore CWD to the initiating repo root after each entry's reconciliation.

For entries without a `project` field, self-heal is unchanged from RFC-2.

2. **Read `.metadata.yaml.outcome` and act on it.**
   - `outcome.outcome == success` and `outcome.phase == merge` → the merge finished but the driver crashed before the terminal plan transition. Run:
     ```bash
     specify plan transition <name> done
     ``` No `status-reason` on success.
   - `outcome.outcome == success` and `outcome.phase ∈ {define, build}` → the phase finished but the driver crashed before launching the next phase. This is **not** a terminal state. Do NOT transition the plan. Fall through to step 3 with `LifecycleStatus ∈ {defined, complete}` to resume the next phase.
   - `outcome.outcome == failure` → run `/spec:drop <name> --reason "<outcome.summary>"` (same drop skill steps 11a/b invoke for a live failure — it is idempotent against an already-dropped change). Then:
     ```bash
     specify plan transition <name> failed --reason "<outcome.summary>"
     ``` `--reason` copied byte-for-byte from `outcome.summary`; never paraphrase or truncate.
   - `outcome.outcome == deferred` → same shape with `blocked`:
     ```bash
     /spec:drop <name> --reason "<outcome.summary>"
     specify plan transition <name> blocked --reason "<outcome.summary>"
     ```
   - The `outcome` field is **absent** and `LifecycleStatus` is non-terminal (`defining`, `defined`, `building`, `complete`) → no terminal outcome was ever stamped; the prior phase was in-flight at crash time. Fall through to step 3 with the on-disk `LifecycleStatus`. This is the explicit "active change dir with no terminal outcome yet" branch from RFC-2 §"Plan Mutation and Crash Safety".
   - The `outcome` field is malformed (unknown enum variant, missing `phase`, missing `summary`, …) or its `phase` contradicts `LifecycleStatus` (for example `phase: merge` with `status: defining`, or `outcome: success` with `status: dropped`) → **halt**. Emit the diagnostic line below with exit code `2` (`EXIT_VALIDATION_FAILED` — see the exit-code table in specify-cli `src/main.rs`). Leave the plan entry as `in-progress`. Do not append a recovery journal entry. Do not drop the change. Humans triage.

3. **Mid-change resume** (no terminal outcome yet). Reached when step 1 found no metadata, when step 2 saw success on `define`/`build`, or when step 2 saw no `outcome` field with a non-terminal `LifecycleStatus`. Read `LifecycleStatus` from `.metadata.yaml.status` (missing metadata counts as "None") and apply the resumption table from RFC-2 §"Context Threading → Resumption Within a Change":
   - `None` → invoke `/spec:define <name>` from scratch (step 6 of the per-change algorithm, skipping step 5 because `plan.yaml` already has the entry `in-progress`).
   - `defining` → resume / restart `/spec:define <name>`.
   - `defined` → invoke `/spec:build <name>` (step 7).
   - `building` → resume `/spec:build <name>`.
   - `complete` → invoke `/spec:merge <name>` (step 8).
   - `merged` or `dropped` → contradiction. **Halt** — same semantics as the step-2 ambiguity branch. Resumption does NOT write a plan transition — the entry stays `in-progress` while the resumed phase sequence completes; the normal step-9 phase-outcome read wraps up the run.

4. **Append one `type: recovery` entry to `journal.yaml`** for every entry self-heal actually resolved or resumed (not for halts):
   ```bash
   specify change journal-append <name> <phase> recovery \
       --summary "Self-heal on startup: <action>" \
       --context "before=<plan-status>/<lifecycle-status>, after=<resolved-status-or-phase>"
   ``` where `<phase>` is the phase the recovery relates to (the `outcome.phase` for terminal cases; the phase about to run for mid-change resume). Example `<action>` strings:
   - `"applied terminal transition done after finding success outcome on merge"`
   - `"applied terminal transition failed after finding failure outcome on build"`
   - `"applied terminal transition blocked after finding deferred outcome on define"`
   - `"resumed mid-change build phase (LifecycleStatus=defined)"`

5. **Lock scope.** Self-heal runs **inside** the driver lock already acquired at step 2 of the outer run. There is no second acquire and no inner release: the whole reconciliation pass is part of the same critical section that wraps the per-change loop. Two `/spec:execute` invocations started at the same time cannot both enter self-heal — the second one fails at `specify plan lock acquire` with `Error::DriverBusy`.

### Diagnostic output

Every self-heal action emits exactly one line to stdout:

```text
Self-heal: no in-progress entries found.
Self-heal: <name> → done (merge success from prior run)
Self-heal: <name> → failed (build failure: "<outcome.summary verbatim>")
Self-heal: <name> → blocked (define deferred: "<outcome.summary verbatim>")
Self-heal: <name> — resuming <phase> (LifecycleStatus=<lifecycle>)
Self-heal halted: <name> has outcome=<outcome> phase=<phase> but LifecycleStatus=<lifecycle>. Manual triage required.
```

The halt variant is followed by `Exit 2` (`EXIT_VALIDATION_FAILED`; see the exit-code table in specify-cli `src/main.rs`); the other variants fall through to step 4 of the per-change algorithm. Fixture-pinned examples live under `fixtures/self-heal/`.

### Dry-run variant (report-only)

Under `--dry-run`, self-heal runs the same classification scan but performs **no writes**: no `specify plan transition`, no `specify change journal-append`, no `/spec:drop`. Instead it prints what the writing path *would* do:

```text
Self-heal (dry-run): no in-progress entries found.
Self-heal (dry-run): <name> → done (if executed) — merge success from prior run
Self-heal (dry-run): <name> → failed (if executed) — build failure: "<outcome.summary verbatim>"
Self-heal (dry-run): <name> → blocked (if executed) — define deferred: "<outcome.summary verbatim>"
Self-heal (dry-run): <name> — would resume <phase> (LifecycleStatus=<lifecycle>)
Self-heal (dry-run): <name> — would halt (ambiguous outcome=<outcome> phase=<phase>, LifecycleStatus=<lifecycle>)
```

Terminal-resolution lines use "(if executed)" to make clear that no transition was written. The halt line still ends the run: dry-run self-heal emits it, releases the lock, and exits non-zero. It is the one place `--dry-run` exits non-zero on the happy startup path — the whole point of the halt is to surface ambiguity. Mid-change resume does nothing in dry-run anyway (the writing path also emits no plan transition for this case). The report-only path never appends `type: recovery` entries — recovery entries are the writing path's observable side-effect; dry-run has no side-effects by contract.

## Modes

Each mode below describes only the *delta* from the §Per-change algorithm. The 13 steps above are the common core; the mode-specific rules are the outer loop, the writing-vs-reporting substitution table, and the interrupt handling.

### Supervised (default)

Run the per-change algorithm once, exactly as written, then exit. A supervised run reaches a terminal plan status (`done` / `failed` / `blocked`) for exactly one change and stops; the driver never iterates to a second change. `--loop` (below) is the mode that layers an outer iteration on top of the same per-change steps.

### `--dry-run`

Run the per-change algorithm with every **write** substituted for a **report** (see §Guardrails for the explicit MUST-NOTs). Concretely:

| Write | Dry-run substitute |
|---|---|
| `specify plan transition <name> in-progress` (step 5) | Not invoked. The preview shows the plan in its current state. |
| `specify plan transition <name> {done,failed,blocked}` (steps 10–12) | Not invoked. Diagnostics use the "Would transition" wording (see self-heal §Dry-run variant). |
| `specify change journal-append … recovery …` (self-heal step 4) | Not invoked. |
| `/spec:define`, `/spec:build`, `/spec:merge`, `/spec:drop` (steps 6–8, 11b, 12b) | Not invoked. `--dry-run` is read-only end to end — the self-heal scan is report-only too. |

Step 4's `specify plan next` / `specify plan status` calls still run — they are read-only. The rendered output follows the §`--dry-run` output format below; every line carries the `[dry-run] ` banner so the operator cannot mistake a preview for a real run.

If self-heal would halt (ambiguity case), dry-run emits the same halt diagnostic, releases the lock, and exits non-zero WITHOUT reaching step 4 — the one non-zero exit on the happy startup path.

### `--loop`

Wrap the per-change algorithm (steps 3–12) in an outer iteration:

```text
4a. Iteration body:
      loop:
        - run steps 3–12 against whatever `specify plan next`
          returns; on return (terminal plan status reached), DO NOT
          release the lock.
        - loop back to step 4 of the per-change algorithm
          (`specify plan next --format json`).
        - break on `reason ∈ {"all-done", "stuck"}` (classifications
          of the same name) or defence-in-depth `reason ==
          "in-progress"` (classification `halted`).
4b. After the loop breaks, emit the terminal summary (§Terminal
    summary) and go to step 13 (lock release) ONCE.
```

Mode invariants:

- **Lock is held for the entire run.** `specify plan lock acquire` runs once at step 2 of the per-change algorithm; `specify plan lock release` runs once at step 13. The outer iteration neither acquires nor releases the lock.
- **Self-heal runs once.** The step-3 pass happens before the first iteration. Subsequent iterations do not re-run self-heal.
- **`failure` does NOT stop the loop.** An individual change that returns `outcome: failure` is transitioned to `failed` inside steps 11a–c; the driver then continues to the next `specify plan next` call. `specify plan next` naturally skips `failed` entries, so the loop advances without extra branching.
- **`deferred` does NOT stop the loop.** Same shape with `blocked` instead of `failed`.
- **Loop stops only when `specify plan next` reports no eligible change.** Terminal classifications are `all-done` (every entry in `{done, skipped}`) or `stuck` (pending / blocked / failed entries remain but no pending entry has its `depends-on` satisfied).
- **`halted` is reserved for self-heal halts.** Mid-loop failures / deferrals do NOT reach `halted`.
- **No phase-level parallelism.** At most one change is `in-progress` at a time; the loop does not fan out concurrent phase invocations.

#### SIGINT / SIGTERM handling

The skill runs inside an agent session; the agent process (not this skill directly) traps SIGINT / SIGTERM. The contract the skill must honour when the agent surfaces an interrupt is:

```text
1. Finish the current PHASE. Do NOT tear a /spec:define, /spec:build,
   or /spec:merge mid-invocation — doing so can leave change
   artifacts in a half-written state that self-heal then has to
   reconcile.

2. Skip subsequent phases of the CURRENT change. If build has not
   yet started when the interrupt arrives, do NOT start it; if
   build has just finished, do NOT invoke /spec:merge. The already-
   completed phase has stamped its outcome on disk, so self-heal on
   the next run will either resume (success on define/build) or
   resolve terminally (success on merge, failure, deferred).

3. Leave the active change entry as in-progress. Do NOT run
   `specify plan transition` on interrupt — the write path is
   reserved for normal outcomes. Self-heal on the next run will
   reclaim the entry based on .metadata.yaml.outcome.

4. Release the driver lock:
     specify plan lock release --pid <agent-session-pid>
   Run this before exit regardless of which phase was mid-flight.

5. Emit the terminal summary with Completion: driver-interrupted and
   Next action pointing the operator at `/spec:execute --loop` to
   resume. The summary's Progress line reflects the state as of the
   interrupt — the active entry still shows in-progress.

6. Exit non-zero (typically 130 for SIGINT, 143 for SIGTERM).
```

The skill cannot trap signals directly (agent-side shells handle signal delivery), but the above is the contract the skill's *logic* must satisfy so that the observable on-disk state after an interrupt is always recoverable by self-heal on the next run.

## Output format

### `--dry-run`

Emits a compact snapshot of the plan's current state and the change the driver *would* run next. The progress line has exactly six status counters in a fixed order so downstream parsers see a stable shape.

```text
[dry-run] /spec:execute — <plan-name>

Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

Next: <name> (sources: [<sources>])

No changes written.
```

Variants the skill picks based on `specify plan next`:

- `reason: "in-progress"` — replace the `Next:` line with: `Active: <name> (driver would resume/adopt this entry)`
- `reason: "all-done"` — replace with: `Initiative complete — no eligible changes remain.`
- `reason: "stuck"` — replace with: `Stuck — no eligible changes. Pending: [<names>]. Blocked: [<names>]. Failed: [<names>].`

The canonical shape is pinned by [`fixtures/dry-run/`](fixtures/dry-run/).

Under multi-repo routing, the `--dry-run` output includes a `Routing:` diagnostic line:

```text
[dry-run] Routing: <name> → <project> (<resolved-path>)
```

### Supervised / per-change transcript

Three variants (success / failure / deferred), each pinned by a behavioural fixture under [`fixtures/single-change/`](fixtures/single-change/).

#### Success

```text
## /spec:execute — <plan-name>

### Initiative: <plan-name>
Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

---

### Processing: <name> (sources: [<sources>])

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

The `(sources: [...])` suffix is rendered only when the plan entry has `sources`; greenfield entries become `### Processing: <name> (greenfield)`. The extract sub-step block inside `Step 1/3: define` is elided when the entry has no `sources`.

#### Failure

```text
## /spec:execute — <plan-name>

### Initiative: <plan-name>
Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

---

### Processing: <name> (sources: [<sources>])

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

The phase that fails is whichever returned `outcome: failure`; the step header names that phase. The `Summary:` line is the phase's `outcome.summary` copied byte-for-byte.

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
  Action needed: Enrich the plan description (specify plan amend …) with the missing
    detail, then unflag (blocked → pending) to retry.
  Status: blocked
```

The `⚠ Question recorded — change deferred to blocked` line is the canonical deferred banner from RFC-2 §"Output and Observability"; do not reword. `Question:` carries the phase's `outcome.summary` verbatim.

### Terminal summary (`--loop` exit)

At the end of every `--loop` run — success, interruption, or halt — `/spec:execute` emits a single terminal summary block. Fixtures under [`fixtures/loop/`](fixtures/loop/) pin one example per `Completion:` value.

```text
## /spec:execute — <plan-name> — terminated

### Final state
Progress: done <N>, in-progress <N>, pending <N>, blocked <N>, failed <N>, skipped <N> (total <N>)

Completion: <all-done | stuck | halted | driver-interrupted>

Blocked:
  - <name> (status-reason: "<verbatim status-reason>")
  - ...

Failed:
  - <name> (status-reason: "<verbatim status-reason>")
  - ...

Pending (dependencies not satisfied):
  - <name> (waits on: <unmet-dep-name>[, <unmet-dep-name> ...])
  - ...

Next action: <context-sensitive instruction>
```

Section rules:

- `Progress:` always enumerates all six statuses in the fixed order `done, in-progress, pending, blocked, failed, skipped`, followed by `(total <N>)`. Zeros are rendered explicitly.
- `Blocked:` / `Failed:` / `Pending (dependencies not satisfied):` sections are omitted entirely (including the heading) when their bucket is empty. An `all-done` run therefore renders only the `Final state` / `Completion` / `Next action` lines.
- `Blocked` and `Failed` entries quote `status-reason` byte-for-byte from the plan entry. Paraphrasing is forbidden.
- `Pending (…)` lists each pending entry alongside the `depends-on` entries whose status is not `done`.
- `in-progress` count is zero on all classifications except `driver-interrupted`, where the active change is preserved as `in-progress` for self-heal to reclaim on the next run.

#### `Completion:` classification

| Classification | Condition | Next action template |
|---|---|---|
| `all-done` | Every entry's status is in `{done, skipped}`. | `Initiative complete — no further action needed.` |
| `stuck` | Some entries remain in `{pending, blocked, failed}` but none are eligible (pending entries have unmet deps; no eligible sibling exists). | `Resolve blocked/failed entries (specify plan amend + specify plan transition <name> blocked → pending / failed → pending) or accept the partial initiative and run specify plan archive --force.` |
| `halted` | Self-heal detected an ambiguous on-disk state on startup and refused to speculate. Individual mid-loop failures or deferrals do NOT reach `halted`. | `Manually triage the halted change: inspect .specify/changes/<name>/.metadata.yaml against plan.yaml, repair the contradiction, then re-run /spec:execute --loop.` |
| `driver-interrupted` | SIGINT or SIGTERM arrived mid-run. The current phase finished (or no phase was in flight), subsequent phases were skipped, the active plan entry is still `in-progress`, the lock was released. | `Re-run /spec:execute --loop — self-heal will reclaim the interrupted change on the next startup.` |

The distinction between `stuck` and `halted` matters for operator routing: `stuck` means the plan is well-formed but needs human-level priority decisions; `halted` means the on-disk state itself is inconsistent and needs forensic triage before the loop can run safely again.

#### Exit codes

| Classification | Exit code |
|---|---|
| `all-done` | 0 |
| `stuck` | 0 (driver did nothing wrong; partial completion is observable via the plan) |
| `halted` | `2` (`EXIT_VALIDATION_FAILED` — see the exit-code table in specify-cli `src/main.rs`) |
| `driver-interrupted` | non-zero (typically 130 for SIGINT, 143 for SIGTERM, inherited from the host shell's signal conventions) |

## What this skill does NOT do (in this Change)

| Surface | Status |
|---|---|
| Write `.specify/plan.yaml` *entries* (`create` / `amend`) | Never — those writes are the phases' concern (they shell out to `specify plan create` / `specify plan amend` mid-run). |
| Write `.specify/plan.yaml` *status* (`transition`) | Only via `specify plan transition`, at exactly three points in a supervised run: `pending → in-progress` before step 6, and the terminal `in-progress → {done, failed, blocked}` in steps 10/11/12. |
| Write `.specify/changes/<name>/.metadata.yaml` (including the `outcome` field) | Never — that is the phase skills' concern via `specify change phase-outcome`. |
| Write `.specify/changes/<name>/journal.yaml` | Only via `specify change journal-append <name> <phase> recovery …` inside the §Self-heal on startup step — exactly one entry per reclaimed or resumed in-progress entry. Phases own the `type: question` / `type: failure` entries and the driver never touches those. |
| Invoke `/spec:define`, `/spec:build`, `/spec:merge`, or `/spec:drop` | Never in `--dry-run` (including dry-run self-heal, which is report-only); in supervised and `--loop` modes, exactly as the algorithms prescribe (define → build → merge on success paths; plus `/spec:drop` on failure / deferred, and on any writing-path self-heal reclaim of a `failure` or `deferred` outcome). |
| Run self-heal on `in-progress` entries | Yes — §Self-heal on startup is the full contract. Five fixtures under `fixtures/self-heal/` pin the clean / done / failed / ambiguous-halt / mid-change-resume paths. Under `--dry-run` self-heal is report-only: same classification scan, no writes. |
| Loop across changes | `--loop` iterates `specify plan next → execute change` until no eligible change remains. The driver lock is held for the entire run (not per iteration). Individual failures / deferrals do NOT halt the loop — `specify plan next` skips `failed` / `blocked` entries naturally. |
| Resolve `sources` keys to paths / URLs and hand them to define | Yes — §Argument resolution resolves every key in the plan entry's `sources` list against the plan's top-level `sources` map and forwards the tuples to `/spec:define` as `--source <key>=<path-or-url>`. The driver does NOT clone git URLs or stat local paths; it only forwards the values. An unresolved key halts the run with `Error::Config`. Scope and delta-targeting intent are carried in the entry's `description` and inferred by the define skill; the driver does not forward them as separate flags. |

The state the skill mutates is:

1. The driver lock stamp at `.specify/plan.lock` (written on acquire, removed on release by the CLI — not by the skill directly).
2. The plan entry's `status` field via `specify plan transition` at the three points named in the per-change algorithm, plus any terminal transitions self-heal applies on startup.
3. A single `type: recovery` entry appended to `.specify/changes/<name>/journal.yaml` via `specify change journal-append` whenever self-heal resolves or resumes an in-progress entry.

No other on-disk state is written by `/spec:execute` itself.

## Guardrails

- Never hand-edit `.specify/plan.yaml`, `.specify/changes/<name>/.metadata.yaml`, or `.specify/changes/<name>/journal.yaml`. Route every write through the CLI verbs above — the single-writer invariant in RFC-2 §"Plan Mutation and Crash Safety" depends on it.
- Never skip the lock-release step. If the skill exits early after a successful acquire, run `specify plan lock release --pid
  <agent-session-pid>` on the way out. Stale stamps can be reclaimed by a later run, but only after a visible-to-the-operator failure.
- Treat an unexpected `specify plan next` response shape (missing keys, unknown `reason`) as a hard failure: print the raw JSON, release the lock, and exit non-zero. Do not speculate.
- For `--dry-run` specifically: the skill MUST NOT invoke any phase skill, MUST NOT shell out to `specify plan transition`, MUST NOT shell out to `specify change journal-append`, and MUST NOT invoke `/spec:drop`. This prohibition extends to the self-heal step: dry-run self-heal is report-only (§Self-heal on startup → §Dry-run variant). The first-line banner prefixes the rendered output with `[dry-run] ` (the progress / next blocks do not need a per-line prefix — the banner is enough).
- For `--loop` specifically: the driver lock is acquired ONCE at run start and released ONCE at run end; never per iteration. Individual change outcomes (success, failure, deferred) are handled inside the iteration body; they do not short-circuit the outer loop. The loop exits only on `specify plan next` reporting no eligible change, self-heal halt on startup, or SIGINT / SIGTERM. On any exit path, the terminal summary is emitted before the lock is released.
- For the supervised single-change run: the string passed to `specify plan transition <name> {failed,blocked} --reason "…"` is always `outcome.summary` from the phase's `.metadata.yaml`, copied byte-for-byte. Never paraphrase, truncate, or add a prefix. The fixtures under `fixtures/single-change/` assert this equality; a drift between `metadata-after-*.yaml:outcome.summary` and `plan.yaml.after:status-reason` is a regression.
- Phase outcome missing or malformed after a phase returns means the phase crashed or skipped its `specify change phase-outcome` call. Treat as `deferred` with a synthetic summary (`"phase outcome missing after <phase>; driver stopping for triage."`) per RFC-2 §"Phase Outcome Contract" — do not speculate about which of success / failure was really intended.
- Self-heal applies the same verbatim-`summary` rule as steps 11c / 12c: the string passed to `specify plan transition <name> {failed,blocked} --reason "…"` is copied byte-for-byte from the on-disk `outcome.summary`. The fixtures under `fixtures/self-heal/` assert this equality; drift is a regression.
- Self-heal never paraphrases ambiguity away. If `.metadata.yaml` has no `outcome`, an `outcome` with a `phase` that contradicts `LifecycleStatus`, or a `LifecycleStatus` that is terminal (`merged`, `dropped`) while `plan.yaml` still says `in-progress`, halt with exit code 1 and leave the plan entry as `in-progress`. A later run, after human triage, can re-enter self-heal safely.
- Argument resolution never speculates over an unresolved `sources` key. If a key on the plan entry is absent from the plan's top-level `sources` map, halt with `Error::Config`, name the offending `(change, key)` pair, release the lock, and exit non-zero. Do NOT substitute a default, guess at a path, or drop the key silently. The same rule applies whether the run is `--dry-run`, supervised, or `--loop`.

## Fixtures

Every behavioural pin for this skill is consolidated here. Each row names the directory and the contracts it pins; deeper entries per directory are named inline.

| Fixture | Pins |
|---|---|
| [`fixtures/dry-run/`](fixtures/dry-run/) | Happy-path `--dry-run` rendering (§`--dry-run` output format). Seed `plan.yaml` paired with the exact rendered `expected-output.md`. |
| [`fixtures/single-change/`](fixtures/single-change/) | Supervised per-change transcripts: success / failure / deferred. Each `plan.yaml.after` carries `status-reason` byte-identical to the phase's `outcome.summary` (drift is a regression). |
| [`fixtures/self-heal/`](fixtures/self-heal/) | Five self-heal paths — clean-start / done-resolution / failed-resolution / ambiguous-halt / mid-change-resume. The writing path copies `outcome.summary` verbatim into `--reason`. |
| [`fixtures/loop/`](fixtures/loop/) | Five `--loop` classifications: `all-done/` (every entry runs to `done`), `halted-on-self-heal-ambiguity/` (self-heal halt on startup), `stuck-on-blocked/` (loop drains eligible entries, exits with an unreachable `blocked` remainder), `driver-busy/` (second invocation refused by the lock), `driver-interrupted/` (SIGINT mid-build; build finishes, merge skipped, entry stays `in-progress`, lock released). |
| [`fixtures/field-wiring/`](fixtures/field-wiring/) | Argument-resolution pins for the two wiring shapes — `sources-only/` (`/spec:define <name> --source monolith=/path/to/legacy`) and `description-driven/` (greenfield or description-inferred entries with no `--source` flags). Each ships `plan.yaml`, `invocation.txt`, and `transcript.md`; these are authoring pins, not automated tests. |
| [`fixtures/e2e-platform-v2/`](fixtures/e2e-platform-v2/) | End-to-end exit-gate meta-fixture: `/spec:execute --loop` driving the full RFC-2 §"The Plan" `platform-v2` example against a plan authored by `/spec:plan`. |
| [`fixtures/e2e-platform-v2-with-crash/`](fixtures/e2e-platform-v2-with-crash/) | Same `platform-v2` plan as above with a simulated mid-change crash; exercises the self-heal-on-startup reclaim path end-to-end. |
| [`fixtures/multi-project/`](fixtures/multi-project/) | Multi-repo CWD routing (RFC-3b): `registry.yaml`, `plan.yaml` with per-entry `project` fields, `execute-loop-transcript.md` pinning the `Routing:` diagnostic and cross-project loop, `workspace-push-output.json` / `workspace-push-dry-run.json` pinning `specify workspace push` output shapes. |
| [`fixtures/greenfield-bootstrap/`](fixtures/greenfield-bootstrap/) | Greenfield `specify workspace sync` fallback sequence: clone-fails → `mkdir` → `git init` → `specify init` → scaffold commit. `partial-rerun/` pins the recovery path when `.git/` exists but `.specify/project.yaml` is absent. |

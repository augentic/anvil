---
name: specify-execute
description: "Drives an initiative through its plan.yaml: reads the plan, picks the next eligible change, runs define → build → merge, and updates status. Use when running the next eligible change in an initiative or processing all eligible changes via `--loop`."
---

## Critical Path (Quick Reference)

1. **Resolve project root** — walk upward from CWD looking for `.specify/project.yaml`; exit non-zero if not found.
2. **Acquire driver lock** — `specify plan lock acquire --pid <agent-session-pid>`. On `DriverBusy`, report and exit.
3. **Self-heal** — reconcile any `in-progress` entries left by a prior crash: read `.metadata.yaml:outcome`, apply terminal transitions or resume mid-change. Halt on ambiguity. See [self-heal.md](self-heal.md).
4. **Pick next change** — `specify plan next --format json`. Handle `all-done` (exit 0), `stuck` (exit 0), or `in-progress` (exit non-zero). Capture `project`, `description`, and `sources` from the response.
5. **Transition to in-progress and route CWD** — `specify plan transition <name> in-progress`. For multi-repo entries, resolve the target project directory from `registry.yaml` and `chdir`. See [multi-repo.md](multi-repo.md).
6. **Run phase sequence** — invoke `/spec:define` → `/spec:build` → `/spec:merge`, reading `.metadata.yaml:outcome` after each phase. On `failure` → drop + transition `failed`. On `deferred` → drop + transition `blocked`. On `registry-amendment-required` (RFC-9 §2B) → record proposal payload to journal → drop + transition `blocked`. Copy `outcome.summary` verbatim into `--reason`.
7. **Wrap up** — transition to `done` on success; on multi-repo successes, run the cross-project contract check (RFC-9 §3B) and append any findings to the merged change's journal as `cross-project-warning:` entries. Release the driver lock on **every** exit path. In `--loop` mode, repeat from step 4 until no eligible change remains, then emit the terminal summary.

The full algorithm lives in [per-change-algorithm.md](per-change-algorithm.md). Mode-specific deltas (`--dry-run`, supervised, `--loop`) live in [modes.md](modes.md). Rendered output shapes live in [output-format.md](output-format.md). Behavioural fixtures pinning each shape live in [fixtures.md](fixtures.md).

# Execute skill

Drive an initiative through `plan.yaml` by automating the Layer 1 loop: `get next change` → `/spec:define` → `/spec:build` → `/spec:merge` (or `/spec:drop`) → `specify plan transition`.

> **Status.** Layer 2 is fully landed. The driver supports multi-repo CWD routing (`project` field on plan entries), `plan next` field extensions (`project`, `description`, `sources` in JSON), workspace status checks, merge auto-commit in workspace clones, and self-heal under multi-repo. This skill ships the `--dry-run` preview, the supervised single-change run, the self-heal pass on startup, `--loop` mode with terminal summary and SIGINT / SIGTERM handling, and the `sources` execution wiring. `/spec:execute --loop` drives the `platform-v2` example end-to-end against a plan authored by `/spec:plan` — see [fixtures.md](fixtures.md) for the exit-gate meta-fixture.

## Overview

Specify at runtime is a three-layer stack:

1. **Phase skills** (`/spec:define`, `/spec:build`, `/spec:merge`, and `/spec:drop`) — the define-build-merge loop that operates on a single change.
2. **Plan CLI** (`specify plan {validate, next, status, create, amend, transition, archive, lock}`) — the library-backed verbs that read and write `plan.yaml`. Both humans (Layer 1) and this skill (Layer 2) drive the loop through these verbs; no other code path writes the plan file.
3. **Driver skill** (`/spec:execute`, this one) — the Layer 2 automation that reads `plan.yaml`, picks the next entry, invokes the phase sequence, and records outcomes.

The on-disk contracts the driver depends on are the same files humans read in Layer 1 — `/spec:execute` introduces no new storage of its own:

| File | Owner | Role |
|---|---|---|
| `plan.yaml` | library (`Plan::{create, amend, transition, archive}`) | Ordered change list with per-entry status. Driver reads via `specify plan next`/`status`; writes only via `specify plan transition`. |
| `.specify/changes/<name>/.metadata.yaml` | library (`ChangeMetadata` + `specify change outcome set`) | Change lifecycle status **and** the phase's `outcome` field. Phases stamp this; the driver reads it on phase return. |
| `.specify/changes/<name>/journal.yaml` | library (`Journal::append` + `specify change journal append`) | Append-only audit log of `question` / `failure` / `recovery` entries. Never consumed as a signalling channel — `.metadata.yaml:outcome` is the only state the driver reads. |
| `.specify/plan.lock` | library (`PlanLockStamp`) | Advisory PID stamp held by the running driver. Prevents two `/spec:execute` invocations racing on the same plan. |

For multi-repo initiatives the driver `chdir`s into a registered project clone under `.specify/workspace/<project>/` before invoking the phase skills. See [multi-repo.md](multi-repo.md) for the routing algorithm and the post-merge cross-project contract check.

## Invariants

These invariants constrain this skill's behaviour.

| Invariant | Enforced by |
|---|---|
| Driver contracts with phases, not briefs | `/spec:execute` only invokes `/spec:define`, `/spec:build`, `/spec:merge` |
| Phases own verify-repair loops | Phase skills exhaust their repair budget before returning |
| Exactly one of `success`/`failure`/`deferred` per phase | Phase writes `outcome` into `.metadata.yaml` before returning |
| Change *entries* written only via `Plan::create` / `Plan::amend` | Phases and humans both run `specify plan add` / `specify plan amend` |
| Change *status* updates written only via `Plan::transition` | `/spec:execute` (Layer 2) or humans (Layer 1) run `specify plan transition` |
| Single `in-progress` at a time | `plan next` / `plan validate` |
| Single `/spec:execute` driver at a time | `.specify/plan.lock` advisory lock (see §Driver lock below) |

## Invocation

```text
/spec:execute              # supervised mode: run one change, stop
/spec:execute --dry-run    # preview next change + progress; no writes
/spec:execute --loop       # run until no eligible change remains
```

The plan path is fixed at `plan.yaml`; multi-plan support is a future capability.

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

## Per-change algorithm at a glance

The full algorithm — including step 9's phase-outcome classifier and the RFC-9 §2B `registry-amendment-required` branch — lives in [per-change-algorithm.md](per-change-algorithm.md). The 13 steps in summary:

1. Resolve project directory (walk upward for `.specify/project.yaml`).
2. Acquire driver lock (`specify plan lock acquire`).
3. Run self-heal ([self-heal.md](self-heal.md)).
4. Pick next change (`specify plan next --format json`); capture `project`, `description`, `sources`.
5. Transition `pending → in-progress` (`specify plan transition`). Route CWD for multi-repo entries ([multi-repo.md](multi-repo.md) §CWD routing).
6. Resolve `sources` ([argument-resolution.md](argument-resolution.md)) and invoke `/spec:define <name>`.
7. On `success`: invoke `/spec:build <name>`.
8. On `success`: invoke `/spec:merge <name>`.
9. Read phase outcome (`specify change outcome show <name> --format json`). Classify `success` / `failure` / `deferred` / `registry-amendment-required` / missing-or-malformed.
9a. Restore CWD for multi-repo entries.
10. On terminal `success`: `specify plan transition <name> done`. Run cross-project contract check ([multi-repo.md](multi-repo.md) §Cross-project).
11. On `failure`: `/spec:drop` + `specify plan transition <name> failed --reason "<outcome.summary>"`.
12. On `deferred` (or `registry-amendment-required`): journal append (RFC-9 §2B path only) → `/spec:drop` + `specify plan transition <name> blocked --reason "<outcome.summary>"`.
13. Release driver lock — on every exit path.

`outcome.summary` is copied byte-for-byte into `--reason` at steps 11c and 12c. Never paraphrase.

## Modes at a glance

| Mode | Behaviour | Detail |
|---|---|---|
| Supervised (default) | Run the per-change algorithm once; exit on terminal status. | [modes.md](modes.md) |
| `--dry-run` | Read-only preview; substitute every write for a report. | [modes.md](modes.md) |
| `--loop` | Iterate until no eligible change remains; emit terminal summary. | [modes.md](modes.md) |

The terminal summary, per-change transcript shapes, and dry-run rendering live in [output-format.md](output-format.md). Behavioural fixtures pinning each mode live in [fixtures.md](fixtures.md).

## Self-heal on startup

Self-heal is the driver's reconciliation pass. It runs **once per `/spec:execute` invocation**, under the driver lock, immediately after the lock is acquired and before `specify plan next`. The full algorithm lives in [self-heal.md](self-heal.md). Key invariants:

- `.metadata.yaml:outcome` is the single authoritative signal — the driver never consults `journal.yaml`, tempfiles, or stderr transcripts.
- Nothing speculates: every ambiguity (missing outcome with no change dir, outcome that contradicts `LifecycleStatus`, …) halts the driver with exit code `2` so a human can triage.
- Self-heal runs inside the lock acquired at step 2; no second acquire/release.
- Under `--dry-run`, self-heal is report-only — same classification scan, no writes.

## Cross-project contract check (RFC-9 §3B)

After a successful merge of a multi-repo change whose plan entry has a non-null `project` field and whose producer registry entry declares non-empty `contracts.produces`, the driver runs the format-appropriate `/contract:*` skill in its verifier intent with `--mode cross-project` against every consumer workspace — `/contract:openapi` for HTTP / resource APIs, `/contract:asyncapi` for evented / pub-sub / streaming, `/contract:json-schema` for shared payload schemas. Findings are appended to the merged change's journal as `cross-project-warning:` entries and rendered in the merge transcript. The check is non-fatal: verifier findings (or even verifier errors) never halt the loop, and the merged change stays `done`. See [multi-repo.md](multi-repo.md) for the full algorithm and the journal payload schema.

## What this skill does NOT do

| Surface | Status |
|---|---|
| Write `plan.yaml` *entries* (`create` / `amend`) | Never — those writes are the phases' concern (they shell out to `specify plan add` / `specify plan amend` mid-run). |
| Write `plan.yaml` *status* (`transition`) | Only via `specify plan transition`, at exactly three points in a supervised run: `pending → in-progress` before step 6, and the terminal `in-progress → {done, failed, blocked}` in steps 10/11/12. |
| Write `.specify/changes/<name>/.metadata.yaml` (including the `outcome` field) | Never — that is the phase skills' concern via `specify change outcome set`. |
| Write `.specify/changes/<name>/journal.yaml` | Three narrowly-scoped paths only: (1) self-heal `recovery` entries — one per reclaimed/resumed in-progress entry; (2) cross-project `cross-project-warning:` entries — one per finding from the format-appropriate `/contract:*` skill (verifier intent, `--mode cross-project`) on a successful multi-repo merge; (3) RFC-9 §2B `registry-amendment-required:` entries — one per `registry-amendment-required` deferral, recorded **before** `/spec:drop`. Phases own all other `type: question` / `type: failure` entries. |
| Invoke `/spec:define`, `/spec:build`, `/spec:merge`, or `/spec:drop` | Never in `--dry-run` (including dry-run self-heal); in supervised and `--loop` modes, exactly as the algorithm prescribes. |
| Run self-heal on `in-progress` entries | Yes — see [self-heal.md](self-heal.md). Five fixtures under `fixtures/self-heal/` pin the clean / done / failed / ambiguous-halt / mid-change-resume paths. |
| Loop across changes | `--loop` iterates `specify plan next → execute change` until no eligible change remains. The driver lock is held for the entire run (not per iteration). Individual failures / deferrals do NOT halt the loop. |
| Resolve `sources` keys to paths / URLs and hand them to define | Yes — see [argument-resolution.md](argument-resolution.md). The driver does NOT clone git URLs or stat local paths; it only forwards the values. |

The state the skill mutates is:

1. The driver lock stamp at `.specify/plan.lock` (written on acquire, removed on release by the CLI — not by the skill directly).
2. The plan entry's `status` field via `specify plan transition` at the three points named in the per-change algorithm, plus any terminal transitions self-heal applies on startup.
3. A single `type: recovery` entry appended to `.specify/changes/<name>/journal.yaml` whenever self-heal resolves or resumes an in-progress entry.
4. One `type: failure` entry per cross-project contract finding appended to the **merged** change's journal after a successful multi-repo `merge` transition. Each entry carries the canonical `cross-project-warning:` summary prefix.

No other on-disk state is written by `/spec:execute` itself.

## Guardrails

- Never hand-edit `plan.yaml`, `.specify/changes/<name>/.metadata.yaml`, or `.specify/changes/<name>/journal.yaml`. Route every write through the CLI verbs above — the single-writer invariant depends on it.
- Never skip the lock-release step. If the skill exits early after a successful acquire, run `specify plan lock release --pid <agent-session-pid>` on the way out. Stale stamps can be reclaimed by a later run, but only after a visible-to-the-operator failure.
- Treat an unexpected `specify plan next` response shape (missing keys, unknown `reason`) as a hard failure: print the raw JSON, release the lock, and exit non-zero. Do not speculate.
- For `--dry-run` specifically: the skill MUST NOT invoke any phase skill, MUST NOT shell out to `specify plan transition`, MUST NOT shell out to `specify change journal append`, and MUST NOT invoke `/spec:drop`. This prohibition extends to the self-heal step: dry-run self-heal is report-only ([self-heal.md](self-heal.md) §Dry-run variant). The first-line banner prefixes the rendered output with `[dry-run] `.
- For `--loop` specifically: the driver lock is acquired ONCE at run start and released ONCE at run end; never per iteration. Individual change outcomes (success, failure, deferred) are handled inside the iteration body; they do not short-circuit the outer loop. The loop exits only on `specify plan next` reporting no eligible change, self-heal halt on startup, or SIGINT / SIGTERM. On any exit path, the terminal summary is emitted before the lock is released.
- For the supervised single-change run: the string passed to `specify plan transition <name> {failed,blocked} --reason "…"` is always `outcome.summary` from the phase's `.metadata.yaml`, copied byte-for-byte. Never paraphrase, truncate, or add a prefix.
- Phase outcome missing or malformed after a phase returns means the phase crashed or skipped its `specify change outcome set` call. Treat as `deferred` with a synthetic summary (`"phase outcome missing after <phase>; driver stopping for triage."`) — do not speculate about which of success / failure was really intended.
- Self-heal applies the same verbatim-`summary` rule as steps 11c / 12c. Self-heal never paraphrases ambiguity away — halt with exit code 2 and leave the plan entry as `in-progress`.
- Argument resolution never speculates over an unresolved `sources` key. If a key on the plan entry is absent from the plan's top-level `sources` map, halt with `Error::Config`, name the offending `(change, key)` pair, release the lock, and exit non-zero.
- The cross-project contract check is **non-fatal by contract**. Any finding — `warning`, `info`, or even a validator read failure — is recorded to the merged change's journal and reflected in the merge transcript, never halting the loop or rolling back the merge.

### When the loop reports `stuck`: run `specify plan doctor`

When `specify plan next` returns `reason: stuck`, or when the terminal summary classifies a `--loop` exit as `stuck`, the operator's first triage step is `specify plan doctor`. `doctor` is a strict superset of `plan validate` (RFC-9 §4B): it surfaces the four health issues `validate` does not catch, each with a stable diagnostic code so dashboards and runbooks can route them mechanically.

| Code | Severity | Meaning | Recovery |
|---|---|---|---|
| `cycle-in-depends-on` | error | Dependency cycle in `depends-on`. `next_eligible` silently skips cycles at runtime; doctor is the only place where the cycle path surfaces. | `specify plan amend <name> --depends-on …` to break the cycle on the offending entry, then re-run `plan doctor`. |
| `orphan-source-key` | warning | Top-level `sources:` key declared but no plan entry references it (the inverse of validate's `unknown-source`). | Either reference the key from an entry's `sources:` list via `specify plan amend <name> --sources …` or remove it from the top-level map. Non-fatal. |
| `stale-workspace-clone` | warning | `.specify/workspace/<project>/` clone's signature has drifted from `registry.yaml`, or no signature is readable. | `specify workspace sync` to refresh the clone. Non-fatal. |
| `unreachable-entry` | error | Pending entry whose dependency closure is rooted in a `failed`/`skipped` predecessor. Distinct from cycles — entries inside a cycle are reported only under `cycle-in-depends-on`. | Recover the predecessor (`specify plan transition <pred> pending` once the underlying issue is fixed) or `specify plan transition <entry> skipped --reason "…"` to drop the leaf. |

The driver itself never invokes `plan doctor` — it is an operator triage verb, not a runtime check. `specify plan validate` continues to be invoked verbatim by `plan next` / `plan status` for the structural-error short-circuit; doctor's four additional codes are surfaced only when the operator asks.

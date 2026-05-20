---
name: change-execute
description: "Drive a change through its plan.yaml on the change surface: read the plan, pick the next eligible slice, run define → build → merge, and update status. Use when running the next eligible slice in a change or processing all eligible slices via `loop`."
---

## Critical Path

1. **Resolve project root** — walk upward from CWD looking for `.specify/project.yaml`; exit non-zero if not found.
2. **Acquire driver lock** — `specify plan lock acquire --pid <agent-session-pid>`. On `DriverBusy`, report and exit.
3. **Self-heal** — reconcile any `in-progress` entries left by a prior crash: read `.metadata.yaml:outcome`, apply terminal transitions or resume mid-slice. Halt on ambiguity. See [self-heal.md](self-heal.md).
4. **Pick next slice** — `specify plan next --format json`. Handle `all-done` (exit 0), `stuck` (exit 0), or `in-progress` (exit non-zero). Capture `project`, `description`, and `sources` from the response.
5. **Prepare workspace entry** — for multi-repo entries, resolve `entry.project` through `registry.yaml`, materialise only the selected slot when missing, and run `specify workspace prepare-branch <project> --change <change-name>` before phase writes. Then transition `pending → in-progress` and `chdir` into the slot. See [multi-repo.md](multi-repo.md).
6. **Run phase sequence** — invoke `/spec:define` → `/spec:build` → `/spec:merge`, reading `.metadata.yaml:outcome` after each phase. On `failure` → drop + transition `failed`. On `deferred` → drop + transition `blocked`. On `registry-amendment-required` → record proposal payload to journal → drop + transition `blocked`. Copy `outcome.summary` verbatim into `reason`.
7. **Wrap up** — after merge success in a workspace slot, verify the baseline commit boundary and commit non-baseline residue as `specify: residue <slice-name>` before `done`. Release the driver lock on **every** exit path. In `loop` mode, repeat from step 4 until no eligible slice remains, then emit the terminal summary. Cross-project consumer-impact reporting is a separate `specify compatibility` CLI surface.

The full algorithm lives in [per-slice-algorithm.md](per-slice-algorithm.md). Shared state-handoff rules live in [execute-state-handoff.md](../../references/execute-state-handoff.md). Mode-specific deltas (`dry-run`, supervised, `loop`) live in [modes.md](modes.md). Rendered output shapes live in [output-format.md](output-format.md). Behavioural fixtures pinning each shape live in [fixtures.md](fixtures.md).

# Execute skill

Drive a change through `plan.yaml` by automating the per-slice loop: `get next slice` → `/spec:define` → `/spec:build` → `/spec:merge` (or `/spec:drop`) → `specify plan transition`.

> **Status.** The driver is fully landed. It supports multi-repo workspace routing (`project` field on plan entries), selected slot materialisation, branch preparation on `specify/<change-name>`, `plan next` field extensions (`project`, `description`, `sources` in JSON), merge-baseline commit verification, residue commits in workspace slots, and self-heal under multi-repo. This skill ships the `dry-run` preview, the supervised single-slice run, the self-heal pass on startup, `loop` mode with terminal summary and SIGINT / SIGTERM handling, and the `sources` execution wiring. `/change:execute loop` drives the `platform-v2` example end-to-end against a plan authored by `/change:draft` — see [fixtures.md](fixtures.md) for the exit-gate meta-fixture.

## Overview

Specify at runtime is a small stack composed of:

1. **Phase skills** (`/spec:define`, `/spec:build`, `/spec:merge`, and `/spec:drop`) — the Layer 1 define-build-merge loop that operates on a single slice.
2. **Plan CLI** (`specify plan {validate, next, status, create, add, amend, transition, archive, lock, doctor}`) — the library-backed `specify` verbs that read and write `plan.yaml`. Both manual operators and this skill drive the loop through these verbs; no other code path writes the plan file.
3. **Driver skill** (`/change:execute`, this one) — the Layer 2 automation that reads `plan.yaml`, picks the next entry, invokes the phase sequence, and records outcomes.

The on-disk contracts are the same files an operator would read when driving the loop manually; `/change:execute` introduces no new storage of its own. The shared state-channel ownership table lives in [execute-state-handoff.md](../../references/execute-state-handoff.md).

For multi-repo changes the driver resolves the plan entry's `project` through `registry.yaml`, materialises that selected slot when missing, prepares `specify/<change-name>` before phase writes, and `chdir`s into the prepared project root before invoking the phase skills. See [multi-repo.md](multi-repo.md) for the routing algorithm and post-merge residue commit. Use `specify compatibility` separately when the operator wants a classified producer-to-consumer contract report.

## Invariants

These invariants constrain this skill's behaviour.

| Invariant | Enforced by |
|---|---|
| Driver contracts with phases, not briefs | `/change:execute` only invokes `/spec:define`, `/spec:build`, `/spec:merge` |
| Phases own verify-repair loops | Phase skills exhaust their repair budget before returning |
| Exactly one of `success`/`failure`/`deferred` per phase | Phase writes `outcome` into `.metadata.yaml` before returning |
| Slice *entries* written only via `Plan::create` / `Plan::amend` | Phases and humans both run `specify plan add` / `specify plan amend`; see [plan-single-writer.md](../../references/plan-single-writer.md) |
| Slice *status* updates written only via `Plan::transition` | `/change:execute` or operators driving the loop manually run `specify plan transition`; see [execute-state-handoff.md](../../references/execute-state-handoff.md) |
| Single `in-progress` at a time | `change plan next` / `change plan validate` |
| Single `/change:execute` driver at a time | `.specify/plan.lock` advisory lock (see §Driver lock below) |

## Invocation

```text
/change:execute              # supervised mode: run one slice, stop
/change:execute dry-run    # preview next slice + progress; no writes
/change:execute loop       # run until no eligible slice remains
```

The plan path is fixed at `plan.yaml`; multi-plan support is a future adapter.

## Driver lock

`/change:execute` takes the `.specify/plan.lock` PID stamp at the start of every run — **including `dry-run`** — and releases it on normal exit. The stamp is managed by three dedicated CLI verbs:

```bash
specify plan lock acquire --pid <agent-session-pid>
specify plan lock status
specify plan lock release --pid <agent-session-pid>
```

Notes on the protocol:

- The stamp is a **PID file with liveness check**, not an `flock(2)`. Short-lived CLI invocations cannot hold an advisory file lock across agent-side work, so the lock is represented as a persistent marker that outlives the `specify` processes writing it. `specify plan lock acquire` reclaims a stale stamp (dead PID, malformed contents) itself before the driver enters the self-heal step; nothing in this skill hand-rolls that check.
- `--pid` defaults to `std::process::id()` of the `specify` binary. `/change:execute` should pass a **stable agent-session PID** on every invocation so `release` can authenticate the holder.
- Another live holder surfaces as `Error::DriverBusy { pid }` (exit code `1`); this skill reports the conflict and stops without touching the plan.
- The long-lived in-process `PlanLockGuard` primitive (with a real `flock`) remains available for any future native driver that keeps a Rust process alive for the full run.

## Modes at a glance

| Mode | Behaviour | Detail |
|---|---|---|
| Supervised (default) | Run the per-slice algorithm once; exit on terminal status. | [modes.md](modes.md) |
| `dry-run` | Read-only preview; substitute every write for a report. | [modes.md](modes.md) |
| `loop` | Iterate until no eligible slice remains; emit terminal summary. | [modes.md](modes.md) |

The terminal summary, per-slice transcript shapes, and dry-run rendering live in [output-format.md](output-format.md). Behavioural fixtures pinning each mode live in [fixtures.md](fixtures.md).

## Self-heal on startup

Self-heal is the driver's reconciliation pass. It runs **once per `/change:execute` invocation**, under the driver lock, immediately after the lock is acquired and before `specify plan next`. The full algorithm lives in [self-heal.md](self-heal.md); the common outcome, journal, and dry-run invariants live in [execute-state-handoff.md](../../references/execute-state-handoff.md).

## Cross-project compatibility report (RM-04)

`/change:execute` does not run the RM-04 compatibility classifier as part of the slice loop. Operators can run `specify compatibility check --change <name> --report-only` (read-only) or `specify compatibility check` (strict gate) after workspace sync or after producer contract changes to compare root `contracts/` against consumer workspace views. RM-11 will decide which classifications become lifecycle gates.

## Guardrails

The state this skill may mutate is limited to the driver lock, plan status transitions, routed workspace Git state, and the driver-owned journal append cases in [execute-state-handoff.md](../../references/execute-state-handoff.md). No other on-disk state is written by `/change:execute` itself; see [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state) for the cross-skill single-writer rules and [plan-single-writer.md](../../references/plan-single-writer.md) for the `plan.yaml` entry-write verbs.

- Route every write to `plan.yaml`, `.specify/slices/<name>/.metadata.yaml`, and `.specify/slices/<name>/journal.yaml` through the CLI verbs above — the single-writer invariant depends on it. See [shared guardrails](../../../references/guardrails.md#single-writer-for-lifecycle-state).
- Never skip the lock-release step. If the skill exits early after a successful acquire, run `specify plan lock release --pid <agent-session-pid>` on the way out. Stale stamps can be reclaimed by a later run, but only after a visible-to-the-operator failure.
- Treat an unexpected `specify plan next` response shape (missing keys, unknown `reason`) as a hard failure: print the raw JSON, release the lock, and exit non-zero. Do not speculate.
- For `dry-run` specifically: the skill MUST NOT invoke any phase skill, MUST NOT shell out to `specify plan transition`, MUST NOT shell out to `specify slice journal append`, and MUST NOT invoke `/spec:drop`. This prohibition extends to the self-heal step: dry-run self-heal is report-only ([self-heal.md](self-heal.md) §Dry-run variant). The first-line banner prefixes the rendered output with `[dry-run] `.
- For `loop` specifically: the driver lock is acquired ONCE at run start and released ONCE at run end; never per iteration. Individual slice outcomes (success, failure, deferred) are handled inside the iteration body; they do not short-circuit the outer loop. The loop exits only on `specify plan next` reporting no eligible slice, self-heal halt on startup, or SIGINT / SIGTERM. On any exit path, the terminal summary is emitted before the lock is released.
- On every `failed`/`blocked` transition (supervised, `loop`, and self-heal alike): the string passed to `specify plan transition <name> {failed,blocked} --reason "…"` is always `outcome.summary` from the phase's `.metadata.yaml`, copied byte-for-byte. Never paraphrase, truncate, or add a prefix.
- For multi-repo entries: `specify workspace prepare-branch <project> --change <change-name>` must pass before any phase skill runs. Never create a branch from a guessed default branch; `origin-head-unresolved` is a hard stop.
- After a routed `/spec:merge` success: never transition the entry to `done` until `.specify/specs/` and `.specify/archive/` are clean and all non-baseline residue is either committed as `specify: residue <slice-name>` or proven clean.
- Phase outcome missing or malformed after a phase returns means the phase crashed or skipped its `specify slice outcome set` call. Treat as `deferred` with a synthetic summary (`"phase outcome missing after <phase>; driver stopping for triage."`) — do not speculate about which of success / failure was really intended.
- Self-heal applies the same verbatim-`summary` rule as the failed/blocked guardrail above. Self-heal never paraphrases ambiguity away — halt with exit code 2 and leave the plan entry as `in-progress`.
- Argument resolution never speculates over an unresolved `sources` key. If a key on the plan entry is absent from the plan's top-level `sources` map, halt with `Error::Config`, name the offending `(slice, key)` pair, release the lock, and exit non-zero.
- Cross-project compatibility reporting is outside `/change:execute`; run `specify compatibility check --change <name> --report-only` (read-only) or `specify compatibility check` (strict gate) when consumer-impact classification is needed.

### When the loop reports `stuck`: run `specify plan validate`

When `specify plan next` returns `reason: stuck`, or when the terminal summary classifies a `loop` exit as `stuck`, the operator's first triage step is `specify plan validate`. `validate` carries the base shape rules plus the four health diagnostics, each with a stable code so dashboards and runbooks can route them mechanically.

| Code | Severity | Meaning | Recovery |
|---|---|---|---|
| `cycle-in-depends-on` | error | Dependency cycle in `depends-on`. `next_eligible` silently skips cycles at runtime; validate is the only place where the cycle path surfaces. | `specify plan amend <name> --depends-on …` to break the cycle on the offending entry, then re-run `change plan validate`. |
| `orphan-source-key` | warning | Top-level `sources:` key declared but no plan entry references it (the inverse of `unknown-source`). | Either reference the key from an entry's `sources:` list via `specify plan amend <name> --sources …` or remove it from the top-level map. Non-fatal. |
| `stale-workspace-clone` | warning | `.specify/workspace/<project>/` clone's signature has drifted from `registry.yaml`, or no signature is readable. | `specify workspace sync` to refresh the clone. Non-fatal. |
| `unreachable-entry` | error | Pending entry whose dependency closure is rooted in a `failed`/`skipped` predecessor. Distinct from cycles — entries inside a cycle are reported only under `cycle-in-depends-on`. | Recover the predecessor (`specify plan transition <pred> pending` once the underlying issue is fixed) or `specify plan transition <entry> skipped --reason "…"` to drop the leaf. |

`change plan next` and `change plan status` short-circuit on the same base shape rules; the four health diagnostics are non-fatal at runtime — they only fail validate's exit code when their severity is `error`. Operators run `change plan validate` ad-hoc as the triage entry point.

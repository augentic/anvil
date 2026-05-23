---
name: specify-execute
description: Drive a reviewed plan through refine → build → merge per entry under an exclusive plan lock. Use when Gate 1 has stamped plan reviewed; not before Gate 1, nor after all entries are done.
---

# Specify Execute

`/spec:execute` is the supervised driver for an operator-stamped `reviewed` plan. It refuses unless `plan.lifecycle == reviewed`, takes an exclusive lock on `.specify/plan.lock`, then loops: ask `specify plan next` for the active entry, route to the right workspace slot, invoke `/spec:refine` → `/spec:build` → `/spec:merge`, and stop the moment a phase fails or the plan drains. No automation flags exist — no `--continue`, no `--one`, no `--until`, no `--dry-run`, no `--yes-plan`. The skill takes no positional arguments; the active plan is the one and only argument.

## Critical Path

1. Verify `plan.lifecycle == reviewed` via `specify plan next`; refuse with the literal `specify plan transition <name> reviewed` hint when the plan is still `pending`.
2. Acquire the exclusive lock on `.specify/plan.lock` (workspace root in workspace mode) using the `flock`-based shell snippet in [`references/plan-lock.md`](references/plan-lock.md); on `plan-lock-busy`, exit immediately with the holder pid.
3. For each `specify plan next` result, route the active slice into its workspace slot when `project` is set, then invoke `/spec:refine` (when the slice is fresh), `/spec:build`, and `/spec:merge` — the only writer of per-entry `done`.
4. Stop on the first build non-zero exit or merge baseline conflict; leave the entry `in-progress` and surface the structured hint from [`references/stop-conditions.md`](references/stop-conditions.md).
5. On `drained` (no `pending` or `in-progress` entries), print the closing hint `drained — run /spec:finalize <name>` and release the lock.
6. Re-entry is implicit: re-running `/spec:execute` after any stop reads `plan.yaml` + slice `.metadata.yaml`, picks up the active `in-progress` entry, and resumes mid-loop — no flags, no resume tokens.

## Refusal gate

`/spec:execute` is illegal before Gate 1. The check is read-only: shell `specify plan next --format json`. The CLI surfaces three terminal cases:

- `error` with discriminant `plan-not-reviewed` — print `specify plan transition <name> reviewed` verbatim and exit non-zero. Never write `reviewed` from this skill; the operator is the only writer of Gate 1.
- A `pending` entry was promoted to `in-progress` (the normal path) — fall through to the loop.
- An `in-progress` entry was already active (re-entry after a stop) — fall through to the loop.
- `drained` — print the closing hint below and exit cleanly without acquiring the lock.

## Plan lock

The lock is an exclusive non-blocking advisory file lock on `.specify/plan.lock` (in workspace mode, on `<workspace-root>/.specify/plan.lock`). Identity is the OS file lock; the lockfile body is informative only (pid, hostname, acquisition timestamp). The full shell snippet — primary `flock -n` path plus the macOS `python3` / `fcntl` fallback — lives in [`references/plan-lock.md`](references/plan-lock.md). Reuse that snippet verbatim in `/spec:refine`, `/spec:build`, and `/spec:merge` when those skills run standalone as breakouts.

On contention (`flock` returns `EWOULDBLOCK`, or the `python3` fallback raises `BlockingIOError`), exit immediately with the structured error `plan-lock-busy` carrying the holder's pid read from the lockfile body. There is no wait, no spin, and no automatic stale-lock removal — when the holder is dead, the operator removes `.specify/plan.lock` by hand. v1 ships no `specify plan lock {acquire,release,status}` CLI verb; the snippet is the only contract.

## Workspace routing

When the active plan entry carries a `project` field, plan artifacts stay at the workspace root and phase work runs in the materialised slot at `.specify/workspace/<project>/`. The routing rules — slot resolution, `specify workspace sync` + `specify workspace prepare`, `chdir` into the slot, residue commit, and CWD restore before the next `specify plan next` — live in [`references/workspace-routing.md`](references/workspace-routing.md). Breakout skills run from the workspace root with the same routing rules: read the active entry, resolve `project`, `chdir` into the slot before phase work, restore CWD before exit.

## Phase invocation

Inside the lock, after routing, sequence the three phase skills against the active `in-progress` entry. Their skill bodies are the authoritative source of phase behaviour; this skill only sequences them and reads their phase outcome from `.metadata.yaml`:

| Phase | Skill body | Trigger |
|---|---|---|
| Refine | [`../refine/SKILL.md`](../refine/SKILL.md) | slice lifecycle is `refining` (or absent — fresh slice). |
| Build | [`../build/SKILL.md`](../build/SKILL.md) | slice lifecycle is `refined`. |
| Merge | [`../merge/SKILL.md`](../merge/SKILL.md) | slice lifecycle is `built` (merge is the sole writer of per-entry `done`). |

When the slice is already past a phase on re-entry (e.g. `refined` after a build-failure stop), skip that phase silently and dispatch to the next one. The phase skills are idempotent by lifecycle and do not write per-entry status themselves; merge alone transitions the plan entry to `done` via `specify slice merge`.

## Stop conditions

Three terminal cases per loop iteration; every other return falls through to the next `specify plan next`. The structured hints are templated in [`references/stop-conditions.md`](references/stop-conditions.md):

- **Build non-zero exit.** Leave the entry `in-progress`. Surface the failing task id and the log path. Hint: *Fix the failure; re-run `/spec:execute` or `/spec:build` to resume from the failed task.*
- **Merge baseline conflict.** Leave the entry `in-progress`. Surface the conflicting baseline paths. Hint: *Resolve the conflict; re-run `/spec:execute` to retry the merge.*
- **Drained.** No `pending` or `in-progress` entries remain. Print the closing hint *drained — run `/spec:finalize <name>`* and exit cleanly. This is the only successful exit.

The lock is released on every exit path by the trailing edge of the snippet's `trap` (or by Python interpreter exit on the macOS fallback).

## Hand-off to `/spec:finalize`

On a clean drain, the closing hint is the literal string `drained — run /spec:finalize <name>` where `<name>` is the plan name from `specify plan next`'s drained envelope. `/spec:finalize` re-validates that every per-entry status is `done`, pushes branches, observes PRs to `MERGED`, then runs `specify plan finalize` to archive. `/spec:execute` does not push branches, does not call `gh`, and does not archive — every one of those side-effects belongs to finalize.

## Guardrails

- **No automation flags.** `--continue`, `--one`, `--until`, `--dry-run`, `--yes-plan`, and any other auto-progression knob is rejected: the skill body takes zero positional arguments and zero flags. The loop is the only mode (RFC-25 D7).
- **Never write `reviewed`.** The plan-lifecycle transition to `reviewed` is operator-only. This skill reads the lifecycle through `specify plan next`; it never shells out to `specify plan transition <name> reviewed`.
- **Never write per-entry `done` directly.** `/spec:merge` is the sole writer of per-entry `done`; this skill only sequences the phase skills.
- **Never skip the lock.** Every shell that runs `specify plan next` or invokes a phase skill must hold the `.specify/plan.lock` exclusive lock — including breakouts of `/spec:refine`, `/spec:build`, and `/spec:merge` when an operator runs them standalone. Reuse the snippet in [`references/plan-lock.md`](references/plan-lock.md).
- **Stop on the first failure.** Build non-zero or merge conflict ends the run; do not advance to the next entry, do not retry the failing phase, do not paper over the stop with a "best-effort" continue.
- **No `gh pr merge`, no branch push, no archive move.** Hand off to `/spec:finalize` on the drained exit; never call the finalize-only side-effects from inside the loop.
- Route every plan-lifecycle and per-entry-status write through the CLI — see [shared guardrails](../../../../docs/standards/skill-guardrails.md#single-writer-for-lifecycle-state).

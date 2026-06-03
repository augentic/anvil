---
name: specify-execute
description: Drive an approved plan through refine → build → merge per entry under an exclusive plan lock. Use when Gate 1 has stamped plan approved; not before Gate 1, nor after all entries are done.
---

# Specify Execute

`/spec:execute` is the supervised driver for an operator-stamped `approved` plan. It refuses unless `plan.lifecycle == approved`, takes an exclusive lock on `.specify/plan.lock`, then loops: ask `specify plan next` for the active entry, route to the right workspace slot, invoke `/spec:refine` → `/spec:build` → `/spec:merge`, and stop the moment a phase fails or the plan drains. No automation flags exist — no `--continue`, no `--one`, no `--until`, no `--dry-run`, no `--yes-plan`. The skill takes no positional arguments; the active plan is the one and only argument.

## Critical Path

1. Verify `plan.lifecycle == approved` via `specify plan next`; refuse with the literal `specify plan transition <name> approved` hint when the plan is still `pending`.
2. Acquire the exclusive lock on `.specify/plan.lock` (workspace in workspace mode) using the `flock`-based shell snippet in [`../../references/plan-lock.md`](../../references/plan-lock.md); on `plan-lock-busy`, exit immediately with the holder pid.
3. For each `specify plan next` result, route the active slice into its workspace slot when `project` is set, then invoke `/spec:refine` (when the slice is fresh), `/spec:build`, and `/spec:merge` — the only writer of per-entry `done`.
4. Stop on the first build non-zero exit or merge baseline conflict; leave the entry `in-progress` and surface the structured hint from [`references/stop-conditions.md`](references/stop-conditions.md).
5. On `drained`, print `drained — run /spec:finalize <name>` and exit — without acquiring the lock when the first `specify plan next` returns drained; otherwise release the lock after the loop.
6. Re-entry is implicit: re-running `/spec:execute` after any stop reads `plan.yaml` + slice `.metadata.yaml`, picks up the active `in-progress` entry, and resumes mid-loop — no flags, no resume tokens.

## Plan lock
Every skill that touches plan state from outside the loop reuses the shell snippet in [`../../references/plan-lock.md`](../../references/plan-lock.md) verbatim. On `plan-lock-busy`, exit immediately with the holder pid read from the lockfile body.

## Workspace routing

When the active plan entry carries a `project` field, plan artifacts stay at the workspace and phase work runs in the materialised slot at `.specify/workspace/<project>/`. The routing rules — slot resolution, `specify workspace sync` + `specify workspace prepare`, `chdir` into the slot, residue commit, and CWD restore before the next `specify plan next` — live in [`references/workspace-routing.md`](references/workspace-routing.md). Breakout skills run from the workspace with the same routing rules: read the active entry, resolve `project`, `chdir` into the slot before phase work, restore CWD before exit.

## Phase invocation

Inside the lock, after routing, sequence the three phase skills against the active `in-progress` entry. Their skill bodies are the authoritative source of phase behaviour; this skill only sequences them and reads slice lifecycle from `.metadata.yaml` and phase exit codes; not an on-disk outcome field.

| Phase | Skill body | Trigger |
|---|---|---|
| Refine | [`../refine/SKILL.md`](../refine/SKILL.md) | slice lifecycle is `refining` (or absent — fresh slice). |
| Build | [`../build/SKILL.md`](../build/SKILL.md) | slice lifecycle is `refined`. |
| Merge | [`../merge/SKILL.md`](../merge/SKILL.md) | slice lifecycle is `built` (merge is the sole writer of per-entry `done`). |
When the slice is already past a phase on re-entry (e.g. `refined` after a build-failure stop), skip that phase silently and dispatch to the next one.

## Guardrails

- **Never write per-entry `done` directly.** `/spec:merge` is the sole writer of per-entry `done`; this skill only sequences the phase skills.
- **Never skip the lock.** Every shell that runs `specify plan next` or invokes a phase skill must hold the `.specify/plan.lock` exclusive lock — including breakouts of `/spec:refine`, `/spec:build`, and `/spec:merge` when an operator runs them standalone. Reuse the snippet in [`../../references/plan-lock.md`](../../references/plan-lock.md).
- **No `gh pr merge`, no branch push, no archive move.** Hand off to `/spec:finalize` on the drained exit; never call the finalize-only side-effects from inside the loop.
- Route every plan-lifecycle and per-entry-status write through the CLI — see [shared guardrails](../../references/guardrails.md#single-writer-for-lifecycle-state).

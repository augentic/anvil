---
name: specify-execute
description: Drive an approved plan through refine → build → merge per entry under an exclusive plan lock. Use when Gate 1 has stamped plan approved; not before Gate 1, nor after all entries are done.
---

# Specify Execute

`/spec:execute` is the supervised driver for an operator-stamped `approved` plan. It is a thin renderer around `specify plan status`: the CLI projects plan entries, slice lifecycle, and the journal tail into a deterministic `next-action`, and this skill takes the lock, routes, invokes the phase skill the CLI named, and surfaces the CLI's stop output verbatim. No automation flags exist — no `--continue`, no `--one`, no `--until`, no `--dry-run`, no `--yes-plan`. The skill takes no positional arguments; the active plan is the one and only argument.

## Critical Path

1. Run `specify plan status --format json`. On `stop plan-not-approved`, refuse with the CLI's hint (it carries the literal `specify plan transition <name> approved` command). On `drained`, print the CLI's `drained — run /spec:finalize <name>` line and exit — without acquiring the lock.
2. Acquire the exclusive lock on `.specify/plan.lock` (the workspace lock in workspace mode, via `--plan-dir`) by driving the loop under `specify plan lock -- <cmd>` per [`../../references/plan-lock.md`](../../references/plan-lock.md); on `plan-lock-busy`, exit immediately with the holder pid. The CLI enforces the same discipline from the other side: `specify plan next`, per-entry `specify plan transition`, and `specify slice merge run` probe the lock and refuse an unlocked driver with `plan-lock-not-held` (exit 2).
3. Loop on `specify plan status --format json` and branch on its `action` field:
   - `refine` / `build` / `merge` — run `specify plan next` (the sole writer of per-entry `in-progress`; it claims the entry when fresh), route the slice into its workspace slot when `project` is set, invoke the named phase skill for `slice`, restore CWD, continue.
   - `stop` — print the CLI's stop block and `hint:` line verbatim (the text rendering matches [`../../references/stop-conditions.md`](../../references/stop-conditions.md)); leave the entry `in-progress` and exit.
   - `drained` — print the CLI's drained line and exit; the loop is the only successful exit path.
4. Re-entry is implicit: re-running `/spec:execute` after any stop asks `specify plan status` again — the CLI picks up the active `in-progress` entry, dispatches on the slice lifecycle (a slice already past a phase resumes at the next one), and re-renders the stop if it still holds. No flags, no resume tokens. The status body also names the re-entry point itself: `resume` carries the literal command or skill invocation that makes progress (rendered as the `resume:` line in text mode), with `current-step` / `last-completed` locating the slice in the loop; surface it with the stop block.

## Plan lock

Every skill that touches plan state from outside the loop drives that work under `specify plan lock -- <cmd>` per [`../../references/plan-lock.md`](../../references/plan-lock.md). On `plan-lock-busy`, exit immediately with the holder pid. Re-entrancy is CLI-owned: the wrapper exports `SPECIFY_PLAN_LOCK_HELD=1`, so a breakout spawned under a parent `/spec:execute` skips re-acquisition automatically. The lock is also runtime-enforced: the plan-state-writing verbs refuse `plan-lock-not-held` (exit 2) when no session holds it.

## Workspace routing

When the active plan entry carries a `project` field, plan artifacts stay at the workspace and phase work runs in the materialised slot at `workspace/<project>/`. The routing rules — slot resolution, `specify workspace sync` + `specify workspace prepare`, `chdir` into the slot, the `SPECIFY_PLAN_DIR=<workspace-root>` export that lets slot-side plan readers resolve the workspace's `plan.yaml`, residue commit, and CWD restore before the next `specify plan status` — live in [`references/workspace-routing.md`](references/workspace-routing.md). Breakout skills run from the workspace with the same routing rules: read the active entry, resolve `project`, `chdir` into the slot before phase work, restore CWD before exit.

## Phase invocation

Inside the lock, after routing, invoke the phase skill named by `plan status` against the active `in-progress` entry. Their skill bodies are the authoritative source of phase behaviour; this skill only renders the CLI's dispatch — it never re-derives the phase from `metadata.yaml` itself.

| `action` | Skill body |
|---|---|
| `refine` | [`../refine/SKILL.md`](../refine/SKILL.md) |
| `build` | [`../build/SKILL.md`](../build/SKILL.md) |
| `merge` | [`../merge/SKILL.md`](../merge/SKILL.md) (merge is the sole writer of per-entry `done`) |

## Guardrails

- **Never write per-entry `done` directly.** `/spec:merge` is the sole writer of per-entry `done`; this skill only invokes the phase skills.
- **Never skip the lock.** Every session that runs `specify plan next` or invokes a phase skill must hold the `.specify/plan.lock` exclusive lock by driving that work under `specify plan lock -- <cmd>` — including breakouts of `/spec:refine`, `/spec:build`, and `/spec:merge` when an operator runs them standalone. See [`../../references/plan-lock.md`](../../references/plan-lock.md); the CLI refuses unlocked drivers with `plan-lock-not-held` and a busy lock with `plan-lock-busy`.
- **Never re-classify stops.** `specify plan status` owns stop classification (`refine-failed`, `build-failed`, `merge-conflict`, `slice-dropped`, `merge-incomplete`, `stuck`); render its block and hint verbatim.
- On any `stop` from `plan status`, print the CLI block verbatim and **exit immediately**; never patch upstream tooling — [Consumer tooling boundary](../../references/guardrails.md#consumer-tooling-boundary).
- **No branch push, no archive move.** Hand off to `/spec:finalize` on the drained exit; never call the finalize-only side-effects from inside the loop.
- Route every plan-lifecycle and per-entry-status write through the CLI — see [shared guardrails](../../references/guardrails.md#single-writer-for-lifecycle-state).

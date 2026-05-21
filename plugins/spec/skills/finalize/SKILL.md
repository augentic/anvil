---
name: specify-finalize
description: "Wrap the post-execute tail of a change: push branches via `specify workspace push`, observe PR state until `MERGED`, then run `specify plan finalize` to archive the plan. Use when every per-entry plan status is `done` and the operator is ready to close the change; not for plan authoring (`/spec:plan`) or per-slice execution (`/spec:execute`)."
argument-hint: <name>
---

# Finalize skill

> **Wrap the post-execute tail of a change.** `/spec:finalize` is composition only over `specify plan next`, `specify workspace push`, `gh pr view`, and `specify plan finalize`. The skill writes nothing under `.specify/` directly — every state mutation is a CLI shell-out, and PR merges stay operator-owned.

## Critical Path

### 1. Pre-flight

Validate `<name>` as kebab-case. Walk upward from CWD for `.specify/project.yaml` to resolve the project (or workspace) root. Verify `plan.yaml` exists at that root.

Pre-flight failures exit non-zero with their own diagnostic — they are not classified halts.

### 2. Drained check via `specify plan next`

Run, from the resolved root:

```bash
specify plan next --format json
```

The plan is drained — every per-entry `status` is `done` — when the envelope reports `reason: drained` (`active: null`, `next: null`). Anything else (an active `in-progress` entry, a pending entry queued, or `stuck`) means at least one entry is still non-`done`. Halt with `non-terminal-entries`, name the offending entry from the envelope (`active` or `next`), and point the operator at `/spec:execute` to drive the loop forward before re-running finalize.

The skill never reads or writes `plan.yaml` directly; drainage is computed by the CLI from the per-entry `status` set.

### 3. Push

Run, from the resolved root:

```bash
specify workspace push
```

The verb pushes the prepared `specify/<name>` branch for every project on the plan, creating PRs through the forge as needed. Single-repo plans are the degenerate case (one project on the table). Workspace plans push every project the plan touches in one invocation. Surface the per-project status table verbatim.

Halt the run on any per-project `failed`, `pending-checks`, or `failed-checks` classification — operator resolves the upstream issue and re-runs `/spec:finalize`.

### 4. PR observation loop

For every pushed project, fetch PR state and poll until `MERGED`. See [`references/runbook.md`](references/runbook.md#step-4--pr-observation-loop) for the verbatim poll body, jitter, and timeout rendering. Summary:

- Initial fetch with `gh pr view <url> --json state,url,number`.
- Any PR `OPEN` → wait the poll interval (default 30s), re-poll. Surface a one-line progress update per cycle.
- Polling ceiling (default 1h). Exhaustion → halt with `pr-poll-exhausted`; the operator merges through the forge UI or a hand-run `gh pr merge`, then re-runs `/spec:finalize`.
- Any PR `CLOSED` (not merged) → halt with `pr-closed`; the operator either reopens or amends the plan, then re-runs.
- Every PR `MERGED` → continue.

The skill never invokes `gh pr merge` itself. PR merges are operator-owned.

### 5. Archive via `specify plan finalize`

Run, from the resolved root:

```bash
specify plan finalize <name>
```

The verb re-validates plan terminality, per-project PR `MERGED` state, and workspace cleanliness, then sweeps `plan.yaml`, `change.md`, and `.specify/plans/<name>/` into `.specify/archive/plans/<name>-<YYYYMMDD>/`. Surface any guard refusal verbatim — most refusals at this step were caught at steps 2–4, but `specify plan finalize` is the canonical guard.

**The skill must never hand-move archive paths.** Archive moves are owned by `specify plan finalize` (and the slice equivalents); this skill defers every write.

After success, print the merged-PR list, the archived plan path, and any post-merge tidy-ups recorded in `change.md`.

## Halts

| Classification | Source step | Operator action |
|---|---|---|
| `non-terminal-entries` | step 2 (`specify plan next`) | run `/spec:execute` until the plan reports `drained`, then re-run |
| `failed` | step 3 (`specify workspace push`) | fix the upstream (auth, network, missing remote), then re-run |
| `pending-checks` / `failed-checks` | step 3 surfaced by `specify workspace push` | wait for / fix the upstream check, then re-run |
| `pr-closed` | step 4 (`gh pr view`) | reopen the PR or amend the plan, then re-run |
| `pr-poll-exhausted` | step 4 (`gh pr view`) | merge each named PR through the forge UI or a hand-run `gh pr merge`, then re-run |
| `specify plan finalize` guard refusals | step 5 | clear the named guard verbatim (operator merges, commits, or re-runs `/spec:execute` as required), then re-run finalize |

Re-entry: each halt re-enters the same skill. Fix the cause, re-run `/spec:finalize <name>`. The skill re-runs `specify plan next` and re-queries every PR on every invocation; nothing tracks "where the operator left off" outside on-disk and remote state.

## Closing message

On success, the skill emits the canonical closing line so peer skills can route on it:

```text
Change <name> finalized. Plan archived at <.specify>/archive/plans/<name>-<YYYYMMDD>/.
```

This is also the literal hand-off target `/spec:execute` prints when it exits because the plan is drained:

```text
Plan drained: every entry is `done`. Run `/spec:finalize <name>` to push branches, observe PRs, and archive the plan.
```

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Verbatim five-step body, halt classifications, polling parameters, re-entry rules |
| [`../execute/SKILL.md`](../execute/SKILL.md) | Peer driver skill that drains the plan this skill closes |
| [`../../../references/guardrails.md`](../../../references/guardrails.md) | Shared single-writer rules every plan-driven skill inherits |

## Guardrails

- **Composition only.** Every step shells out to `specify plan next`, `specify workspace push`, `gh pr view`, or `specify plan finalize`. The skill writes nothing under `.specify/` and merges no PRs itself.
- **Drainage is computed, not stored.** Always go through `specify plan next` — never read `plan.yaml` directly to decide whether the plan is drained.
- **Halts are surfaced verbatim.** Per-project `failed`, per-project `pending-checks` / `failed-checks`, every `gh pr view` row that is not `MERGED`, and every `specify plan finalize` guard diagnostic flow through unchanged.
- **No PR merge automation.** Step 4 observes only; the operator merges PRs through the forge UI or a hand-run `gh pr merge`. This skill never invokes `gh pr merge`.
- **No archive hand-moves.** `specify plan finalize` is the sole writer for archive paths; no `mv` into `.specify/archive/` ever appears in this skill.
- **No new on-disk state.** The skill defers every mutation to CLI verbs. Halt classifications and recovery sequences come from the underlying verbs; this skill does not invent its own.

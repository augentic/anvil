---
name: change-finalize
description: "Wrap the post-execute tail of a change: push branches via `specify workspace push`, observe PR state via `gh pr list`, then run `specify change finalize` once every PR is `MERGED`. Use when every plan entry is terminal and the operator is ready to close out the change; not for authoring (`/change:draft`) or per-slice execution (`/change:execute`)."
argument-hint: <change-name> [dry-run]
---

# Finalize skill

> **Wrap the post-execute tail of a change.** `/change:finalize` is composition only over `specify workspace push`, `gh pr list`, and `specify change finalize`. The skill writes nothing under `.specify/` directly — every state mutation is a CLI shell-out, and PR merges stay operator-owned.

## Critical Path

1. **Pre-flight** — validate `<change-name>` as kebab-case; resolve the project root by walking upward for `.specify/project.yaml`; verify `plan.yaml` exists alongside it.
2. **Plan terminality** — read `plan.yaml`; halt with `non-terminal-entries` (pointing the operator at `/change:execute loop`) if any entry is `pending` or `in-progress`. Terminal statuses are `done`, `failed`, `blocked`, and `skipped`.
3. **Push** — `specify workspace push`. Surface the per-project status table verbatim. Halt with `failed` on any per-project failure; halt with `pending-checks` / `failed-checks` when the verb surfaces those classifications.
4. **PR observation** — `gh pr list --head specify/<change-name> --state all --json number,state,merged,headRefName,url` for each pushed project. Halt with `pr-not-merged` if any PR is not `MERGED`, naming each open PR with its URL. The skill never merges PRs.
5. **Finalize** — `specify change finalize`. Surface guard refusals verbatim (plan absent, non-terminal entries, dirty workspace, unmerged PR). Most are caught upstream; the verb is the canonical guard.
6. **Wrap-up summary** — print the merged-PR list, the archived plan path, and any post-merge tidy-ups recorded in `change.md`.

See [`references/runbook.md`](references/runbook.md) for the verbatim step bodies, halt classifications, re-entry rules, and `--dry-run` rendering.

## Invocation

```text
/change:finalize <change-name>           # supervised: push, observe PRs, finalize
/change:finalize <change-name> dry-run   # observation-only: report plan terminality + would-push branches + PR state
```

## Halts

The skill emits exactly these halt classifications. Nothing is invented or paraphrased.

| Classification | Source step | Operator action |
|---|---|---|
| `non-terminal-entries` | step 2 | run `/change:execute loop` until every plan entry is terminal, then re-run |
| `failed` | step 3 (`specify workspace push`) | fix the upstream (auth, network, missing remote), then re-run |
| `pending-checks` / `failed-checks` | step 3 surfaced by `specify workspace push` | wait for / fix the upstream check, then re-run |
| `pr-not-merged` | step 4 | merge each named PR through the forge UI or a hand-run `gh pr merge`, then re-run |
| finalize CLI guard refusals | step 5 (`specify change finalize`) | clear the named guard verbatim — operator merges, commits, or re-runs execute as required, then re-runs finalize |

Re-entry: each halt re-enters the same skill. Fix the cause, re-run `/change:finalize <change-name>`. The skill re-reads `plan.yaml` and remote PR state on every invocation; nothing tracks "where the operator left off" outside on-disk and remote state.

## `dry-run` semantics

`dry-run` is observation-only:

- reads `plan.yaml` and reports terminality per entry;
- enumerates the branches `specify workspace push` would push (no push executed);
- runs `gh pr list` (read-only) and reports each PR's state;
- never invokes `specify workspace push`;
- never invokes `specify change finalize`;
- writes nothing under `.specify/`.

A non-terminal plan or any non-`MERGED` PR is reported in the preview but does not exit non-zero — `dry-run` is information-only.

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Verbatim six-step body, halt classifications, re-entry rules, `--dry-run` output shape |
| [`fixtures/`](fixtures/) | Per-halt regression fixtures: `terminal-status-not-met/`, `push-failed/`, `pr-not-merged/`, `finalize-guard-refusal/`, `happy-path/` |
| [`../execute/SKILL.md`](../execute/SKILL.md) | Peer driver skill that produces the terminal `plan.yaml` this skill consumes |
| [`../../../references/guardrails.md`](../../../references/guardrails.md) | Shared single-writer rules every change skill inherits |

## Guardrails

- **Composition only.** Every step shells out to `specify workspace push`, `gh pr list`, or `specify change finalize`. The skill writes nothing under `.specify/` and merges no PRs itself.
- **Halts are surfaced verbatim.** Per-project `failed`, per-project `pending-checks` / `failed-checks`, every `gh pr list` row that is not `MERGED`, and every `specify change finalize` guard diagnostic flow through unchanged.
- **No PR merge automation.** Step 4 observes only; the operator merges PRs through the forge UI or a hand-run `gh pr merge`. The skill never invokes `gh pr merge` itself.
- **`dry-run` is read-only.** No `specify workspace push`, no `specify change finalize`, no writes under `.specify/`, no PR mutation.
- **Plan terminality is checked, not enforced.** `specify change finalize` re-validates every guard at step 5; the upstream check at step 2 exists so the skill can name `non-terminal-entries` before any push happens, not to substitute for the CLI's authoritative guard.
- **No new on-disk state.** The skill defers every mutation to CLI verbs. Halt classifications and recovery sequences come from the underlying verbs; this skill does not invent its own.

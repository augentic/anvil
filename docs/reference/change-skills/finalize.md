# /change:finalize

Push branches, observe PR state, and run `specify change finalize` once every PR is `MERGED`.

`/change:finalize` is the close-out stage of the three-skill change lifecycle (`/change:draft → operator review → /change:execute loop → /change:finalize`). It wraps the post-execute tail — push, PR observation, archive — and never merges PRs itself.

The skill lives at [`plugins/change/skills/finalize/SKILL.md`](../../../plugins/change/skills/finalize/SKILL.md); the runbook with the verbatim step bodies, halt classifications, re-entry rules, and `--dry-run` rendering lives at [`plugins/change/skills/finalize/references/runbook.md`](../../../plugins/change/skills/finalize/references/runbook.md).

## Synopsis

```text
/change:finalize <change-name>           # supervised: push, observe PRs, finalize
/change:finalize <change-name> dry-run   # observation-only: report plan terminality + would-push branches + PR state
```

## Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `change-name` | Yes | Kebab-case name of the change to close out |
| `dry-run` | No | Observation-only; reports plan terminality, would-push branches, and PR state without writing |

## When to use

- Every plan entry is in a terminal status (`done`, `failed`, `blocked`, or `skipped`) — see [`/change:execute`](execute.md).
- The implementation has been reviewed and you are ready to push branches, watch PRs merge, and archive the change.

## Lifecycle position

```text
/change:draft  →  operator review  →  /change:execute loop  →  /change:finalize
```

`/change:finalize` is the peer of `/change:draft` and `/change:execute` — three skills, one rhythm. Re-entry is by re-running the same skill: fix the cause (merge a PR, re-push a branch, run more execute) and invoke `/change:finalize <change-name>` again.

## Critical Path

The skill runs a fixed six-step body. See [`plugins/change/skills/finalize/references/runbook.md`](../../../plugins/change/skills/finalize/references/runbook.md) for the verbatim step text.

1. **Pre-flight** — validate `<change-name>` as kebab-case; resolve the project root by walking upward for `.specify/project.yaml`; verify `plan.yaml` exists alongside it.
2. **Plan terminality** — read `plan.yaml`; halt with `non-terminal-entries` (pointing the operator at `/change:execute loop`) if any entry is `pending` or `in-progress`. Terminal statuses are `done`, `failed`, `blocked`, and `skipped`.
3. **Push** — `specify workspace push`. Surface the per-project status table verbatim. Halt with `failed` on any per-project failure; halt with `pending-checks` / `failed-checks` when the verb surfaces those classifications.
4. **PR observation** — `gh pr list --head specify/<change-name> --state all --json number,state,merged,headRefName,url` for each pushed project. Halt with `pr-not-merged` if any PR is not `MERGED`, naming each open PR with its URL. The skill never merges PRs and never wraps `gh pr list` in a CLI verb — the `gh` shell-out is direct.
5. **Finalize** — `specify change finalize`. Surface guard refusals verbatim (plan absent, non-terminal entries, dirty workspace, unmerged PR). Most are caught upstream; the verb is the canonical guard.
6. **Wrap-up summary** — print the merged-PR list, the archived plan path, and any post-merge tidy-ups recorded in `change.md`.

## Halt classifications

The skill emits exactly these halt classifications. Nothing is invented or paraphrased.

| Classification | Source step | Operator action |
|---|---|---|
| `non-terminal-entries` | step 2 | run `/change:execute loop` until every plan entry is terminal, then re-run |
| `failed` | step 3 (`specify workspace push`) | fix the upstream (auth, network, missing remote), then re-run |
| `pending-checks` | step 3 surfaced by `specify workspace push` | wait for the upstream check to complete, then re-run |
| `failed-checks` | step 3 surfaced by `specify workspace push` | fix the failing check, then re-run |
| `pr-not-merged` | step 4 | merge each named PR through the forge UI or a hand-run `gh pr merge`, then re-run |
| finalize CLI guard refusals | step 5 (`specify change finalize`) | clear the named guard verbatim — operator merges, commits, or re-runs execute as required, then re-runs finalize |

Re-entry: each halt re-enters the same skill. The skill re-reads `plan.yaml` and remote PR state on every invocation; nothing tracks "where the operator left off" outside on-disk and remote state.

## `dry-run` semantics

`dry-run` is observation-only:

- reads `plan.yaml` and reports terminality per entry;
- enumerates the branches `specify workspace push` would push (no push executed);
- runs `gh pr list` (read-only) and reports each PR's state;
- never invokes `specify workspace push`;
- never invokes `specify change finalize`;
- writes nothing under `.specify/`.

A non-terminal plan or any non-`MERGED` PR is reported in the preview but does not exit non-zero — `dry-run` is information-only.

## Guardrails

- **Composition only.** Every step shells out to `specify workspace push`, `gh pr list`, or `specify change finalize`. The skill writes nothing under `.specify/` and merges no PRs itself.
- **No PR merge automation.** Step 4 observes only; the operator merges PRs through the forge UI or a hand-run `gh pr merge`.
- **Halts surface verbatim.** Per-project `failed`, per-project `pending-checks` / `failed-checks`, every non-`MERGED` `gh pr list` row, and every `specify change finalize` guard diagnostic flow through unchanged.

## See also

- [/change:draft](draft.md) — author the `plan.yaml` that `/change:finalize` closes out.
- [/change:execute](execute.md) — drive plan entries to terminal status before finalizing.
- [`specify change finalize`](../cli/change.md#specify-change-finalize) — the CLI verb the skill wraps.
- [`specify workspace push`](../cli/workspace.md) — the push verb invoked at step 3.

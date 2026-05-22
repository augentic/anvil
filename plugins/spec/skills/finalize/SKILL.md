---
name: specify-finalize
description: "Wrap the post-execute tail of a change: push branches via `specify workspace push`, observe PR state until `MERGED`, then run `specify plan finalize` to archive the plan. Use when every per-entry plan status is `done` and the operator is ready to close the change; not for plan authoring (`/spec:plan`) or per-slice execution (`/spec:execute`)."
argument-hint: <name>
---

# Finalize skill

> **Wrap the post-execute tail of a change.** `/spec:finalize` is composition only over `specify plan next`, `specify workspace push`, `gh pr view`, and `specify plan finalize`. The skill writes nothing under `.specify/` directly — every state mutation is a CLI shell-out, and PR merges stay operator-owned.

## Critical Path

- Pre-flight: validate `<name>`, resolve the project or workspace root, and verify the active `plan.yaml`.
- Drainage: run `specify plan next --format json`; only `reason: drained` may continue.
- Push: run `specify workspace push` and surface the per-project status table verbatim.
- Observe PRs: poll each pushed PR with `gh pr view <url> --json state,url,number` until every PR is `MERGED`.
- Archive: run `specify plan finalize <name>`, then print merged PRs, the archive path, and post-merge tidy-ups.

Follow [`references/runbook.md`](references/runbook.md) for the verbatim five-step body, halt classifications, polling parameters, re-entry rules, and guard discipline.

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
| [`references/runbook.md`](references/runbook.md) | Verbatim five-step body, halt classifications, polling parameters, re-entry rules, and guard discipline |
| [`../execute/SKILL.md`](../execute/SKILL.md) | Peer driver skill that drains the plan this skill closes |
| [`../../../references/guardrails.md`](../../../references/guardrails.md) | Shared single-writer rules every plan-driven skill inherits |

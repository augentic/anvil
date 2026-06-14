---
name: specify-finalize
description: "Wrap the post-execute tail of a change: push the prepared branches via `specify workspace push`, then run `specify plan archive` to archive the plan. Use when every per-entry plan status is `done` and the operator is ready to close the change; not for plan authoring (`/spec:plan`) or per-slice execution (`/spec:execute`)."
argument-hint: <name>
---

# Finalize skill

> **Wrap the post-execute tail of a change.** `/spec:finalize` is composition only over `specify plan status`, `specify workspace push`, and `specify plan archive`. The skill writes nothing under `.specify/` directly — every state mutation is a CLI shell-out. Pull-request creation and merging are entirely operator-owned and happen outside Specify; the skill neither creates, observes, nor merges PRs.

## Critical Path

- Pre-flight: validate `<name>`, resolve the project or workspace, and verify the active `plan.yaml`.
- Drainage: run `specify plan status --format json`; only `action: drained` may continue. (Read-only — never `plan next`, which is a lock-gated plan-state writer.)
- Push: run `specify workspace push` and surface the per-project status table verbatim.
- Archive: run `specify plan archive`, then print the pushed branches, the archive path, the reminder to open PRs by hand, and any post-merge tidy-ups.

Follow [`references/runbook.md`](references/runbook.md) for the verbatim four-step body, halt classifications, re-entry rules, and guard discipline.

## Closing message

On success, the skill emits the canonical closing line so peer skills can route on it:

```text
Change <name> finalized. Plan archived at <.specify>/archive/plans/<name>-<YYYYMMDD>.yaml.
```

## Reference Documentation

| Reference | Purpose |
|---|---|
| [`references/runbook.md`](references/runbook.md) | Verbatim four-step body, halt classifications, re-entry rules, and guard discipline |
| [`../execute/SKILL.md`](../execute/SKILL.md) | Peer driver skill that drains the plan this skill closes |
| [`references/guardrails.md`](../../references/guardrails.md) | Shared single-writer rules every plan-driven skill inherits |

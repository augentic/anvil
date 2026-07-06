---
name: specify-finalize
description: "Wrap the post-execute tail of a change: verify the plan is drained, push the prepared branches via `specify workspace push`, then archive the plan via `specify plan archive`. Use when every per-entry plan status is `done`; not for plan authoring (`/spec:plan`) or per-slice execution (`specify plan execute`)."
argument-hint: <name>
---

# Finalize Skill

Composition only, over three CLI verbs — the skill writes nothing under `.specify/` directly, and pull-request creation and merging stay operator-owned outside Specify.

## Invocation

1. **Drainage gate** — run `specify plan status`; only `drained` may continue (it is read-only — never substitute `specify plan next`, a plan-state writer). On any other projection, surface the status output verbatim and stop.
2. **Push** — run `specify workspace push` and surface the per-project status table verbatim.
3. **Archive** — run `specify plan archive` and surface the archive path.

## Relay

- On success, close with the canonical line and a reminder to open PRs by hand:

```text
Change <name> finalized. Plan archived at <.specify>/archive/plans/<name>-<YYYYMMDD>.yaml.
```

- On non-zero exit at any step, surface the structured error verbatim and stop; re-running re-enters cleanly. Route every state write through the CLI — see [shared guardrails](../../references/guardrails.md#single-writer-for-lifecycle-state).

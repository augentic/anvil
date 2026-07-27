---
name: emery-finalize
description: "Wrap the post-publication tail of a change: verify the plan is drained, confirm operator-owned publication is complete, then archive via `emery plan archive`. Use when every per-entry plan status is `done`; not for plan authoring (`/emery:plan`) or per-slice execution (`emery plan execute`)."
argument-hint: <name>
---

# Finalize Skill

Composition only — the skill writes nothing under `.emery/` directly, and branch publication, pull-request creation, and merging stay operator-owned outside Emery.

## Invocation

1. **Drainage gate** — run `emery plan status`; only `drained` may continue (it is read-only — never substitute `emery plan next`, a plan-state writer). On any other projection, surface the status output verbatim and stop.
2. **Publication gate** — ask the operator to confirm that affected repositories have been committed, published, and completed through their required review/merge workflow. If not confirmed, stop without archiving.
3. **Archive** — run `emery plan archive` and surface the archive path.

## Relay

- On success, close with the canonical line:

```text
Change <name> finalized. Plan archived at <.emery>/archive/plans/<name>-<YYYYMMDD>.yaml.
```

- On non-zero exit at any step, surface the structured error verbatim and stop; re-running re-enters cleanly. Route every state write through the CLI — it is the single writer for lifecycle state.

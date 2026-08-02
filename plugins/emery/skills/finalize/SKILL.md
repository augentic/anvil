---
name: emery-finalize
description: "Wrap the post-publication tail of a change: verify the plan is drained, confirm operator-owned publication is complete, then archive via `emery plan archive`. Use when every per-entry plan status is `done`; not for plan authoring (`/emery:plan`) or per-slice execution (`emery plan execute`)."
argument-hint: <name>
---

# Finalize Skill

Composition only — the skill writes nothing under `.emery/` directly, and branch publication, pull-request creation, and merging stay operator-owned outside Emery.

## Invocation

1. **Drainage gate** — run `emery plan status --quiet`; only `drained` may continue (it is read-only — never substitute `emery plan advance`, which writes plan state). On any other status, surface the output verbatim and stop.
2. **Publication gate** — ask the operator to confirm that affected repositories have been committed, published, and completed through their required review/merge workflow. If not confirmed, stop without archiving.
3. **Archive** — run `emery plan archive --quiet`. Both verbs are short deterministic operations — they run with `--quiet` per the plugin rule's *Tracing and output* contract (`--debug` replaces it when the operator asks for debug).

## Relay

- On success, relay the archive verb's output verbatim, including the archive path it reports — do not compose a replacement closing line.
- On non-zero exit at any step, surface the structured error verbatim and stop; re-running re-enters cleanly. Route every state write through the CLI — it is the single writer for lifecycle state.

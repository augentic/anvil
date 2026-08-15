---
name: emery-finalize
description: "Wrap the post-publication tail of a change: verify the plan is drained, confirm operator-owned publication is complete, then archive via `emery plan archive` — which itself verifies the publication set against the forge. Use when every per-entry plan status is `done`; not for plan authoring (`/emery:plan`) or execution (`emery plan execute`)."
argument-hint: <name>
---

# Finalize Skill

Composition only — the skill writes nothing under `.emery/` directly, and branch publication, pull-request creation, and merging stay operator-owned outside Emery (the execute drain materializes each publication member's `change/<plan>` worktree; the operator commits with both `Emery-Change` trailers, pushes, and opens the pull requests).

## Invocation

1. **Drainage gate** — run `emery plan status --quiet` (read-only); only `drained` may continue. On any other status, surface the output verbatim and stop.
2. **Publication gate** — ask the operator to confirm that every publication member has been committed, pushed, and merged through its required review workflow. If not confirmed, stop without archiving. (This gate is a courtesy check — archive verifies the same thing authoritatively by reading the forge.)
3. **Archive** — run `emery plan archive --quiet`. Archive observes the forge: it refuses `publication-unverified` (naming every failing member) until each member's pull request carries both trailers and has merged in dependency order. On that refusal, relay the failing members and stop — do not pass `--unverified` unless the operator explicitly asks to bypass verification (the bypass is journaled). Both verbs are short deterministic operations — they run with `--quiet` per the plugin rule's *Tracing and output* contract (`--debug` replaces it when the operator asks for debug). When the Cursor workspace is not the change home, elicit `--change-dir` and pass it on both verbs.

## Relay

- On success, relay the archive verb's output verbatim, including the archive path it reports — do not compose a replacement closing line.
- On non-zero exit at any step, surface the structured error verbatim and stop; re-running re-enters cleanly. Route every state write through the CLI — it is the single writer for lifecycle state.

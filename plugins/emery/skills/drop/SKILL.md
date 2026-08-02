---
name: emery-drop
description: Drop a slice without merging its specs into the baseline by invoking `emery slice drop` and relaying its output. Use when an in-progress slice must be abandoned and archived without folding its deltas into the baseline — the rollback counterpart to `merge`.
argument-hint: "[slice-name] [reason]"
---

# Drop Skill

`emery slice drop` owns the whole drop — the lifecycle transition (refusing terminal `merged` / `dropped` slices), the `dropped-at` and `drop-reason` stamps, and the archive move. This skill only selects the slice, confirms, invokes the verb, and relays its output.

## Invocation

```bash
emery slice drop <slice-name> --reason "<rationale>" --quiet
```

Drop is a short deterministic verb — it runs with `--quiet` per the plugin rule's *Tracing and output* contract (`--debug` replaces it when the operator asks for debug).

- When `[slice-name]` is omitted, run `emery slice list --quiet` and let the operator pick with the AskQuestion tool (confirm even when only one exists).
- Always confirm the drop with the AskQuestion tool before running — a drop archives the slice without merging and is not undoable from the skill. When no `reason` argument was supplied, elicit the rationale as part of that confirmation; warn first when the slice is `built`, since `/emery:merge` may be the intended action.

## Relay

- Surface the CLI output verbatim, including the `archive-path` destination.
- On non-zero exit, surface the structured error verbatim and stop. Never merge or rewrite anything under `.emery/specs/`, and never move the slice directory by hand — the CLI is the single writer for lifecycle state.

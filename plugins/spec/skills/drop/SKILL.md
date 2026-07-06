---
name: specify-drop
description: Drop a slice without merging its specs into the baseline by invoking `specify slice drop` and relaying its output. Use when an in-progress slice must be abandoned and archived without folding its deltas into the baseline — the rollback counterpart to `merge`.
argument-hint: "[slice-name] [reason]"
---

# Drop Skill

`specify slice drop` owns the whole drop — the lifecycle transition (refusing terminal `merged` / `dropped` slices), the `dropped-at` and `drop-reason` stamps, and the archive move. This skill only selects the slice, confirms, invokes the verb, and relays its output.

## Invocation

```bash
specify slice drop <slice-name> --reason "<rationale>"
```

- When `[slice-name]` is omitted, enumerate `.specify/slices/*/` and let the operator pick with the AskQuestion tool (confirm even when only one exists).
- When invoked interactively (no `reason` argument), elicit the rationale and confirm the drop with the AskQuestion tool before running; warn first when the slice is `built`, since `/spec:merge` may be the intended action. When `reason` is supplied, skip the confirmations and run directly.

## Relay

- Surface the CLI output verbatim, including the `archive-path` destination.
- On non-zero exit, surface the structured error verbatim and stop. Never merge or rewrite anything under `.specify/specs/`, and never move the slice directory by hand — see [shared guardrails](../../references/guardrails.md#single-writer-for-lifecycle-state).

---
name: specify-merge
description: Merge a built slice by invoking `specify slice merge run` and relaying its output. Use when landing one slice as a breakout after `/spec:build` succeeds; the `specify plan execute` loop runs the same verb itself. Not when the slice is still `refining` / `refined`, or has already merged.
argument-hint: "[slice-name]"
---

# Merge Skill

`specify slice merge run` owns the whole landing — baseline conflict detection, the deterministic delta merge, Decision Record promotion, the `merged` transition, the archive move, the `slice.archive.created` ledger entry, and the per-entry `done` stamp (it is the sole writer of `done`). This skill only resolves the slice name, confirms, invokes the verb, and relays its output.

## Invocation

```bash
specify slice merge run <slice-name>
```

When `[slice-name]` is omitted, run `specify plan status` and use the slice it names for the `merge` action; if the plan projects no merge action, surface the status output and stop. When invoked interactively, confirm with the AskQuestion tool before running (`specify slice merge preview <slice-name>` renders the read-only preview when the operator asks to see it first; `specify slice merge conflict-check <slice-name>` probes baseline drift).

## Relay

- Surface the CLI output verbatim, including the archive path and the promoted `decisions[]` ids.
- On non-zero exit (e.g. a baseline conflict), surface the structured error verbatim and stop; the slice stays `built` and the plan entry stays `in-progress`. Never auto-revert a landed merge and never move anything into `.specify/archive/` by hand — see [shared guardrails](../../references/guardrails.md#single-writer-for-lifecycle-state).

---
name: emery-merge
description: Merge a built slice by invoking `emery slice merge` and relaying its output. Use when landing one slice as a breakout after `/emery:build` succeeds; the `emery plan execute` loop runs the same verb itself. Not when the slice is still `refining` / `refined`, or has already merged.
argument-hint: "[slice-name]"
---

# Merge Skill

`emery slice merge` owns the whole landing — the target's preflight gate, baseline conflict detection, the deterministic delta merge, promoting decision records into the baseline, the `merged` transition, the archive move, the target's postflight gate, the archive journal entry, and the per-entry `done` stamp (it is the sole writer of `done`). This skill only resolves the slice name, confirms, invokes the verb, and relays its output.

## Invocation

```bash
emery slice merge <slice-name>
```

The committed merge is a long-running orchestration — it runs bare (or with `--debug` when the operator asks) per the plugin rule's *Tracing and output* contract; the dry-run flags below run with `--quiet`.

When `[slice-name]` is omitted, run `emery plan status --quiet` and use the slice it names for the `merge` action; if the status names no merge action, surface the status output and stop. When invoked interactively, confirm with the AskQuestion tool before running (`emery slice merge <slice-name> --preview --quiet` renders the read-only preview when the operator asks to see it first; `emery slice merge <slice-name> --conflict-check --quiet` probes baseline drift — both dry-run flags write nothing).

## Relay

- Surface the CLI output verbatim, including the archive path and the promoted `decisions[]` ids.
- On a non-zero exit there are two cases, and the difference is whether the merge had already committed:
  - **Failed before the commit** (preflight gate failure or baseline conflict): nothing landed. The slice stays `built` and the plan entry stays `in-progress` — fix the reported problem, then re-run `/emery:merge`.
  - **Failed after the commit** (the postflight gate): the merge already landed. The slice is `merged` and archived (its `merge/postflight.yaml` records `status: failure`) and the plan entry is `done` — there is nothing to retry for this slice. Relay the diagnostic; the operator repairs the baseline (hand-fix or a follow-up slice), then runs `/emery:execute` to acknowledge the stop and continue the plan.
- Never auto-revert a landed merge and never move anything into `.emery/archive/` by hand — the CLI is the single writer for lifecycle state.

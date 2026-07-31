---
name: emery-merge
description: Merge a built slice by invoking `emery slice merge run` and relaying its output. Use when landing one slice as a breakout after `/emery:build` succeeds; the `emery plan execute` loop runs the same verb itself. Not when the slice is still `refining` / `refined`, or has already merged.
argument-hint: "[slice-name]"
---

# Merge Skill

`emery slice merge run` owns the whole landing — the target's preflight gate, baseline conflict detection, the deterministic delta merge, Decision Record promotion, the `merged` transition, the archive move, the target's postflight gate, the `slice.archive.created` ledger entry, and the per-entry `done` stamp (it is the sole writer of `done`). This skill only resolves the slice name, confirms, invokes the verb, and relays its output.

## Invocation

```bash
emery slice merge run <slice-name>
```

When `[slice-name]` is omitted, run `emery plan status` and use the slice it names for the `merge` action; if the plan projects no merge action, surface the status output and stop. When invoked interactively, confirm with the AskQuestion tool before running (`emery slice merge run <slice-name> --preview` renders the read-only preview when the operator asks to see it first; `emery slice merge run <slice-name> --conflict-check` probes baseline drift — both dry-run flags write nothing).

## Relay

- Surface the CLI output verbatim, including the archive path and the promoted `decisions[]` ids.
- On non-zero exit before the commit (a preflight gate failure or baseline conflict), surface the structured error verbatim and stop; the slice stays `built` and the plan entry stays `in-progress`, so re-running after a fix re-enters cleanly. A postflight failure is terminal-but-merged: the slice is already `merged` and archived with `merge/postflight.yaml` (including `status: failure`), the plan entry is `done`, and `emery plan status` sticks on `stop merge-postflight-failed` until `emery plan execute` acknowledges and continues — relay the diagnostic without attempting any rollback or retrying `/emery:merge` for that archived slice. Never auto-revert a landed merge and never move anything into `.emery/archive/` by hand — the CLI is the single writer for lifecycle state.

---
name: emery-correct
description: Record a durable operator correction for a decomposition domain by invoking `emery plan correct` and relaying its output. Use when `emery plan author` parked a domain after a failed cut (`plan-author-stopped` / `stop partition-parked`), or to re-cut a domain on an authored plan into an amendment proposal.
argument-hint: [domain] [intent]
---

# Correct Skill

The CLI orchestration owns the whole correction — parked-vs-authored phase split, constraint enforcement, the `plan.correction.recorded` fact, and the boundary proposal on the authored path. This skill elicits the arguments, invokes one verb, and relays its output.

## Invocation

```bash
emery plan correct [--domain <id>] [--constraint <close-as-leaf|split>] [--child <id>]... --intent "…" [--change-dir <dir>]
```

Elicit the operator's intent verbatim — one or two sentences stating how the domain should cut (an ordering, a boundary, a merge). Pass it through unchanged as `--intent`.

- `--domain` may be omitted when exactly one domain is parked; otherwise elicit the id (`plan status` and the stop card name parked domains).
- Offer `--constraint` only when the operator states a structural directive: `close-as-leaf` (the domain is one slice) or `split` (optionally with `--child <id>` per required child). Free-text intent alone is model guidance.
- The command inherits the Cursor workspace cwd as the change root. When that is not the change home, elicit `--change-dir` and pass it through.

## Relay

- Surface the command's output verbatim.
- `correction recorded` (parked author): the fact alone — point the operator at re-running `emery plan author`, which honors the correction on re-entry.
- `correction proposed` (authored plan): relay the printed `emery plan amend --proposal <digest>` line as-is; live planning artifacts stay unchanged until the operator applies it.
- On `plan-correction-non-reducing`, the re-cut would uncover or fail to reduce its domain — relay the error verbatim; the live tree is unchanged and the operator can restate the intent.
- On any other non-zero exit, surface the structured error verbatim and stop.

---
name: emery-status
description: Report where the plan stands and the literal next command by invoking the read-only `emery plan status` and relaying its output. Use whenever the operator asks "where are we", "what's next", or wants to re-orient mid-change; it writes nothing.
---

# Status Skill

`emery plan status` is the read-only projection of the plan's execution state: the plan header, per-entry counts, the deterministic `next-action`, and a literal `resume:` command. This skill only invokes the verb and relays its output.

## Invocation

```bash
emery plan status --quiet
```

Status is a short deterministic verb — it runs with `--quiet` per the plugin rule's *Tracing and output* contract (`--debug` replaces it when the operator asks for debug).

## Relay

- Surface the CLI output verbatim. The `resume:` line is the answer to "what do I do next" — a skill invocation (`/emery:execute`, `/emery:finalize <name>`) or a literal command (usually `emery plan execute`, possibly with `--waive` selectors).
- On a stop projection (`stop: <reason>` with `hint:`), the hint already names the recovery path — relay it without improvising an alternative.
- On non-zero exit (for example `artifact-not-found` when no `plan.yaml` exists), surface the structured error and its hint verbatim and stop — the hint names the authoring entry point (`/emery:plan`).
- Never write anything in response to a status probe: no status edits, no artifact changes.

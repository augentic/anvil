---
name: emery-execute
description: Execute a plan by invoking the `emery plan execute` loop and relaying its output. Use after `/emery:plan` exits, or to resume a partially executed plan; not for per-slice breakouts (`/emery:refine`, `/emery:build`, `/emery:merge`).
---

# Execute Skill

The CLI orchestration owns the whole loop — the run lock, refine → build → merge per entry, and every stop it reports. Running `emery plan execute` on an authored plan is itself the operator's approval; there is no separate approval step, state, or confirmation. This skill invokes the loop and relays its output.

## Invocation

```bash
emery plan execute
```

The loop is a long-running orchestration — it runs bare (or with `--debug` when the operator asks) per the plugin rule's *Tracing and output* contract.

## Relay

- Surface the loop's output verbatim. On drain it prints the completed phases and the canonical `drained — run /emery:finalize <name>` closing line — relay it as-is without adding another pointer.
- On `plan-execute-stopped` (exit 2), the loop already prints the canonical stop card (`stop: <reason>` / `hint:` / `resume:`) on stdout beside the error envelope — relay both verbatim; the resume line names the matching breakout (`/emery:refine`, `/emery:build`, `/emery:merge`) or the re-entrant `emery plan execute`. No follow-up `emery plan status` call is needed.
- On any other non-zero exit, surface the structured error verbatim and stop; re-running re-enters cleanly. Workspace plans refuse execution (`plan-execute-workspace-unsupported`) — drive them hand-driven via `emery plan advance` and the breakouts.
- Route every state write through the CLI — never hand-edit `plan.yaml`.

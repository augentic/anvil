---
name: emery-execute
description: Approve and execute a plan by gating Gate 1 on explicit operator confirmation, then invoking the guest-routed `emery plan execute` loop (whose first run stamps `approved`) and relaying its output. Use after `/emery:plan` exits at `pending`, or to resume an already-approved plan; not for per-slice breakouts (`/emery:refine`, `/emery:build`, `/emery:merge`).
---

# Execute Skill

The engine guest owns the whole drained loop — the Gate 1 stamp (`pending → approved` on the first run), the guest lock, claim → refine → build → merge per entry, and every stop classification. This skill carries the operator's Gate 1 decision behind an explicit confirmation, then invokes the loop and relays its output. `/emery:plan` never runs execute; this skill runs it only when the operator says so.

## Invocation

1. **Status probe** — run `RUST_LOG=off emery plan status` (read-only — never substitute `emery plan next`, a plan-state writer). Branch on the header's `plan: <name> (pending|approved)` line and the `next-action` projection:
   - lifecycle `pending` — continue to step 2 (the first execute will stamp Gate 1).
   - `drained` — surface the output verbatim (its `resume` line already names `/emery:finalize`) and stop.
   - lifecycle `approved` with any dispatch or stop projection — skip to step 3.
2. **Gate 1 (approval gate)** — ask the operator to confirm they have reviewed `change.md`, `discovery.md`, and `plan.yaml` and approve the plan. Without an explicit affirmative, stop without running anything — never infer approval from context; invoking `emery plan execute` on a `pending` plan is the approval act.
3. **Execute**:

```bash
RUST_LOG=info,opentelemetry=off,opentelemetry_sdk=off,omnia_wasi_otel=off \
  emery plan execute
```

The loop is a long-running orchestration — the `RUST_LOG` prefix (and the debug variant) follows the plugin rule's *Tracing and output* contract. Leave `--actor` at its default (`operator`): the stamp relays the operator's explicit confirmation, not the agent's judgment.

## Relay

- Surface the loop's output verbatim. On drain it prints the `approved:` stamp line (first run only), the completed phases, and the canonical `drained — run /emery:finalize <name>` closing line — relay it as-is without adding another pointer.
- On `plan-execute-stopped` (exit 2), relay the structured error verbatim, then run `RUST_LOG=off emery plan status` and surface its canonical stop card (`stop: <reason>` / `hint:` / `resume:`) — the resume line names the matching breakout (`/emery:refine`, `/emery:build`, `/emery:merge`) or the re-entrant `emery plan execute`.
- On any other non-zero exit, surface the structured error verbatim and stop; re-running re-enters cleanly. Workspace plans refuse execution (`plan-execute-workspace-unsupported`) — drive them hand-driven via `emery plan next` and the breakouts.
- Route every state write through the CLI — never hand-edit `plan.yaml` or stamp lifecycle yourself.

---
name: specify-execute
description: Approve and execute a plan by gating Gate 1 on explicit operator confirmation, then invoking the guest-routed `specify plan execute` loop and relaying its output. Use after `/spec:plan` exits at `pending`, or to resume an already-approved plan; not for per-slice breakouts (`/spec:refine`, `/spec:build`, `/spec:merge`).
---

# Execute Skill

The engine guest owns the whole drained loop — the approved-plan gate, the guest lock, claim → refine → build → merge per entry, and every stop classification. This skill carries the operator's Gate 1 decision behind an explicit confirmation, then invokes the loop and relays its output. `/spec:plan` never stamps `approved`; this skill stamps it only when the operator says so.

## Invocation

1. **Status probe** — run `specify plan status` (read-only — never substitute `specify plan next`, a plan-state writer). Branch on the projection:
   - `stop plan-not-approved` (plan still `pending`) — continue to step 2.
   - `lifecycle: approved` — skip to step 3.
   - `drained` — surface the output, point at `/spec:finalize`, and stop.
   - any other `stop <reason>` — surface the output verbatim and stop.
2. **Gate 1 (approval gate)** — ask the operator to confirm they have reviewed `change.md`, `discovery.md`, and `plan.yaml` and approve the plan. Without an explicit affirmative, stop without writing anything — never infer approval from context. On confirmation, run:

```bash
specify plan transition <plan-name> approved
```

`<plan-name>` is the `plan` field from the status body. Leave `--actor` at its default (`operator`): the stamp relays the operator's explicit confirmation, not the agent's judgment.

3. **Execute**:

```bash
specify plan execute
```

## Relay

- Surface the loop's output verbatim. It exits on `drained` (every entry `done` — point at `/spec:finalize`) or on the first `stop <reason>`; on a stop, relay the classification and the `resume` hint from `specify plan status`, and offer the matching breakout (`/spec:refine`, `/spec:build`, `/spec:merge`, `/spec:drop`).
- On non-zero exit, surface the structured error verbatim and stop; re-running re-enters cleanly. Workspace plans refuse execution (`plan-execute-workspace-unsupported`) — drive them hand-driven via `specify plan next` and the breakouts.
- Route every state write through the CLI — never hand-edit `plan.yaml` or stamp lifecycle yourself.

---
name: emery-execute
description: Approve and execute a plan by gating Gate 1 on explicit operator confirmation, then invoking the guest-routed `emery plan execute` loop and relaying its output. Use after `/emery:plan` exits at `pending`, or to resume an already-approved plan; not for per-slice breakouts (`/emery:refine`, `/emery:build`, `/emery:merge`).
---

# Execute Skill

The engine guest owns the whole drained loop — the approved-plan gate, the guest lock, claim → refine → build → merge per entry, and every stop classification. This skill carries the operator's Gate 1 decision behind an explicit confirmation, then invokes the loop and relays its output. `/emery:plan` never stamps `approved`; this skill stamps it only when the operator says so.

## Invocation

1. **Status probe** — run `emery plan status` (read-only — never substitute `emery plan next`, a plan-state writer). Branch on the `next-action` projection — not the plan header's `(pending|approved)` lifecycle label (text mode never emits a `lifecycle: approved` token):
   - `stop plan-not-approved` (plan still `pending`) — continue to step 2.
   - `drained` — surface the output, point at `/emery:finalize`, and stop.
   - `refine|build|merge <slice>` — skip to step 3.
   - any other `stop <reason>` — surface the output verbatim and stop.
2. **Gate 1 (approval gate)** — ask the operator to confirm they have reviewed `change.md`, `discovery.md`, and `plan.yaml` and approve the plan. Without an explicit affirmative, stop without writing anything — never infer approval from context. On confirmation, run:

```bash
emery plan approve
```

Leave `--actor` at its default (`operator`): the stamp relays the operator's explicit confirmation, not the agent's judgment.

3. **Execute**:

```bash
emery plan execute
```

## Relay

- Surface the loop's output verbatim. It exits on `drained` (every entry `done` — point at `/emery:finalize`) or on the first `stop <reason>`; on a stop, relay the classification and the `resume` hint from `emery plan status`, and offer the matching breakout (`/emery:refine`, `/emery:build`, `/emery:merge`, `/emery:drop`).
- On non-zero exit, surface the structured error verbatim and stop; re-running re-enters cleanly. Workspace plans refuse execution (`plan-execute-workspace-unsupported`) — drive them hand-driven via `emery plan next` and the breakouts.
- Route every state write through the CLI — never hand-edit `plan.yaml` or stamp lifecycle yourself.

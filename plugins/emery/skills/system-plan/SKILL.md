---
name: emery-system-plan
description: Project a definition home's migration plan by invoking `emery system plan` and relaying its output. Use after `/emery:system-survey` to propose the initial target architecture and migration plan, or to reproject views and wave handoffs after any definition edit; re-running is resume.
argument-hint: [dir]
---

# System Plan Skill

The CLI orchestration owns the whole run — the one-time initial-plan proposal judgment (only while `system.yaml` has no `target`), architecture view reprojection with digest stamps, and the content-addressed `handoffs/<digest>.yaml` projection per wave. This skill invokes the verb and relays its output.

## Invocation

```bash
emery system plan
```

The plan is a long-running orchestration when the proposal judgment runs — it runs bare (or with `--debug` when the operator asks) per the plugin rule's *Tracing and output* contract. Forward a non-CWD definition home as `--dir <home>`.

## Relay

- Surface the CLI output verbatim: whether the initial architecture was proposed, the reprojected states, and each wave's handoff digest — that digest is what `/emery:system-review` takes.
- After the first run, `system.yaml`'s `target` / `transition-*` states and `migration.yaml` are operator-owned — later runs only reproject; relay `proposed: false` output without suggesting a re-proposal.
- On `system-model-missing`, the definition has not surveyed — relay the hint pointing at `emery system survey`.
- Never hand-edit anything under `architecture/` or `handoffs/` — projections are engine-generated and digest-stamped; an edit is staleness, and re-running the plan replaces it.

---
name: emery-execute
description: Execute a plan by invoking the `emery plan execute` loop and relaying its output. Use after `/emery:refine` has produced fresh refinement manifests, or to resume a partially executed or stopped plan — re-running execute is the only resume path; there are no per-slice breakout commands.
---

# Execute Skill

The CLI orchestration owns the whole loop — the run lock, the `plan.execute.started` authorization epoch at start (covering the exact per-slice refinement digests), the gap gate before build, build → merge per entry, and every stop it reports. Execute never refines: a missing or stale refinement manifest is a typed `plan-refinement-required` stop pointing at `emery plan refine`. There is no separate `plan approve` verb or projected `approved` rung. This skill invokes the loop and relays its output.

## Invocation

```bash
emery plan execute
```

The loop is a long-running orchestration — it runs bare (or with `--debug` when the operator asks) per the plugin rule's *Tracing and output* contract.

When the gap inventory blocks with open gaps the operator chooses to defer, record the durable per-requirement deferrals the stop card names, then re-invoke the loop (deferral facts survive resumes and fresh epochs — nothing is re-supplied):

```bash
emery plan defer <slice>/<req> --reason "<why>"
emery plan execute
```

Alternatively, relay an operator-requested one-epoch policy override (`emery plan execute --gap-policy defer` dispositions every open gap at the build gate) — `--gap-policy` is just another argument to elicit and pass through.

## Relay

- Surface the loop's output verbatim. On drain it prints the completed phases and the canonical `drained — run /emery:finalize <name>` closing line — relay it as-is without adding another pointer.
- On `plan-execute-stopped` (exit 2), the loop already prints the canonical stop card (`stop: <reason>` / `hint:` / `resume:`) on stdout beside the error envelope — relay both verbatim; the resume line is `emery plan execute` (fix the reported problem — or defer named gaps with `emery plan defer` — then re-run; the loop resumes at the parked phase), or `emery plan refine` (on `refinement-required` — refresh the manifests first). No follow-up `emery plan status` call is needed.
- On any other non-zero exit, surface the structured error verbatim and stop; re-running re-enters cleanly.
- Route every state write through the CLI — never hand-edit `plan.yaml`.

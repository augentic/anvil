---
name: emery-plan
description: Plan an Emery change by invoking the `emery plan author` orchestration and relaying its output. Use when starting a fresh change from a reviewed handoff; not when continuing an existing plan (run `emery plan refine` or `emery plan execute`).
argument-hint: <name> --from <definition-home> --wave <id>
---

# Plan Skill

The CLI orchestration owns wave binding and decomposition — import the reviewed handoff, ingest locators, decompose the catalog, and publish `decomposition.yaml` + `plan.yaml`. This skill only elicits arguments, confirms replace when a plan already exists, invokes the verb, and relays its output.

## Invocation

1. **Replace gate** — when `plan.yaml` already exists at the plan root **and the operator asked to replace it**, confirm with the AskQuestion tool that they want the wholesale replace (rewrites `plan.yaml` and `discovery.yaml` — the existing plan is rebound to the same reviewed handoff). On affirmative, pass `--force` in step 2. `--force` is never the recover path: re-running bare `emery plan author` on an incomplete or parked plan resumes the open and parked domains, and on a reconciled plan it is a read-only no-op. Skip this step when `plan.yaml` is absent. A changed wave needs a new handoff and review fact; `--force` will refuse it.
2. **Author**:

```bash
emery plan author <name> --from <definition-home> --wave <id>
# when replacing an existing plan after step 1:
emery plan author <name> --force --from <definition-home> --wave <id>
```

Authoring is a long-running orchestration — it runs bare (or with `--debug` when the operator asks) per the plugin rule's *Tracing and output* contract.

- `<name>` is the kebab-case change name (the CLI rejects malformed names).
- `--from` is the reviewed definition home. Relative values join the product root in-place (`.emery/system/` for a colocated degenerate definition) or the change home when detached.
- `--wave` is the wave id inside that definition.
- Intent arrives only through the handoff. There is no `--intent` or `--source` flag.
- Pass `--force` only after the step-1 confirmation (or when the operator supplied it explicitly).

## Relay

- Surface the CLI output verbatim, including the projected `slices[]`. This skill never runs refine or execute itself — authoring exits so the operator can review the topology before `emery plan refine`.
- On `plan-author-stopped` (exit 2), authoring parked one or more domains after failed cuts — the stop card on stdout names them beside the error envelope; relay both verbatim. The resume path is re-running this skill's verb bare — never `--force`.
- On any other non-zero exit, surface the structured error verbatim and stop. Never hand-edit `plan.yaml`, `change.md`, `discovery.yaml`, `leads.md`, or `decomposition.yaml` — the CLI is the single writer for plan state.
- When the Cursor workspace is not the change home, elicit `--change-dir` and pass it through. Detached homes have no ancestor walk: cwd (or `--change-dir`) *is* the change root.
- Headless plan curation stays on the CLI: `emery plan add`, `emery plan amend`, `emery plan remove`, `emery plan drop`.

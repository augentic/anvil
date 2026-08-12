---
name: emery-plan
description: Plan an Emery change by invoking the `emery plan author` orchestration and relaying its output. Use when starting a fresh change from one or more bound sources (or pure intent); not when continuing an existing plan (run `emery plan refine` or `emery plan execute`).
argument-hint: <name> [source]...
---

# Plan Skill

The CLI orchestration owns the whole authoring flow — plan scaffold, per-source survey into `discovery.md`, lead reconciliation into `slices[]`, review prose, and validation. This skill only elicits arguments, confirms replace when a plan already exists, invokes the verb, and relays its output.

## Invocation

1. **Replace gate** — when `plan.yaml` already exists at the plan root, confirm with the AskQuestion tool that the operator wants to replace it (rewrites `plan.yaml`, `change.md`, and the discovery preamble — the existing plan is recreated whatever its entry statuses, and is not archived). Without an explicit affirmative, stop without running anything. On affirmative, pass `--force` in step 2. Skip this step when `plan.yaml` is absent.
2. **Author**:

```bash
emery plan author <name> --source <key>=<adapter>:<binding>
# when replacing an existing plan after step 1:
emery plan author <name> --force --source <key>=<adapter>:<binding>
```

Authoring is a long-running orchestration — it runs bare (or with `--debug` when the operator asks) per the plugin rule's *Tracing and output* contract.

- `<name>` is the kebab-case change name (the CLI rejects malformed names).
- Forward each operator-supplied `source <key>=<adapter>:<binding>` positional as one repeated `--source` flag. `<key>` is an operator-chosen label — it becomes the slot name in `plan.yaml.sources` that plan entries and evidence files reference (e.g. `legacy`, `docs`). `<binding>` is a path (`documentation:./design-notes/identity`) or the literal form `value:<literal>` (`intent:value:fix typo in user.rs`). Worked example: `--source legacy=typescript:./legacy --source docs=documentation:./design-notes`.
- When the operator passes no source tokens, elicit a one-line intent and pass it as `--intent "<one-line intent>"` instead of `--source`.
- Pass `--force` only after the step-1 confirmation (or when the operator supplied it explicitly).

## Relay

- Surface the CLI output verbatim, including the closing hint (the literal `emery plan refine` command). This skill never runs refine or execute itself — authoring exits so the operator can review `change.md`, `discovery.md`, and `plan.yaml`, then continue through `/emery:refine` or by running `emery plan refine` directly.
- On non-zero exit, surface the structured error verbatim and stop. Never hand-edit `plan.yaml`, `change.md`, or `discovery.md` — the CLI is the single writer for plan state.
- Headless plan curation stays on the CLI: `emery plan add`, `emery plan amend`, `emery plan remove`, `emery plan drop`.

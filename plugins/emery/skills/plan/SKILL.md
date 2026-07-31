---
name: emery-plan
description: Plan a Emery change by invoking the guest-routed `emery plan author` orchestration and relaying its output. Use when starting a fresh change from one or more bound sources (or pure intent); not when continuing an already-approved plan (run `emery plan execute`).
argument-hint: <name> [source]...
---

# Plan Skill

The engine guest owns the whole authoring flow — plan scaffold, per-source survey into `discovery.md`, lead reconciliation into `slices[]`, Gate 1 review prose, validation, and the exit at `pending`. This skill only elicits arguments, confirms replace when a plan already exists, invokes the verb, and relays its output.

## Invocation

1. **Replace gate** — when `plan.yaml` already exists at the plan root, confirm with the AskQuestion tool that the operator wants to replace the pending plan (rewrites `plan.yaml`, `change.md`, and the discovery preamble). Without an explicit affirmative, stop without running anything. On affirmative, pass `--force` in step 2. Skip this step when `plan.yaml` is absent. An approved or in-flight plan cannot be replaced — surface `plan-author-not-replaceable` and point at `emery plan archive` instead of retrying with `--force`.
2. **Author**:

```bash
emery plan author <name> --source <key>=<adapter>:<binding>
# when replacing a pending plan after step 1:
emery plan author <name> --force --source <key>=<adapter>:<binding>
```

Authoring is a long-running orchestration — it runs bare (or with `--debug` when the operator asks) per the plugin rule's *Tracing and output* contract.

- `<name>` is the kebab-case change name (the CLI rejects malformed names).
- Forward each operator-supplied `source <key>=<adapter>:<binding>` positional as one repeated `--source` flag. `<binding>` is a path (`documentation:./design-notes/identity`) or the literal form `value:<literal>` (`intent:value:fix typo in user.rs`).
- When the operator passes no source tokens, elicit a one-line intent and pass it as `--intent "<one-line intent>"` instead of `--source`.
- Pass `--force` only after the step-1 confirmation (or when the operator supplied it explicitly).

## Relay

- Surface the CLI output verbatim, including the closing Gate 1 hint (the literal `emery plan execute` command). Never run execute yourself — Gate 1 is operator-only; invoking `emery plan execute` on a `pending` plan is the approval act, and the operator runs it directly or through `/emery:execute`'s explicit approval gate.
- On non-zero exit, surface the structured error verbatim and stop. Never hand-edit `plan.yaml`, `change.md`, or `discovery.md` — the CLI is the single writer for lifecycle state.
- Headless Gate 1 curation stays on the CLI: `emery plan add`, `emery plan amend`, `emery plan remove`.
- Workspace plans cannot run under `emery plan execute` (`plan-execute-workspace-unsupported`); drive them hand-driven instead — `emery plan next`, then the `/emery:refine` → `/emery:build` → `/emery:merge` breakouts per slice.

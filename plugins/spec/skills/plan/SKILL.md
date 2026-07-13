---
name: specify-plan
description: Plan a Specify change by invoking the guest-routed `specify plan author` orchestration and relaying its output. Use when starting a fresh change from one or more bound sources (or pure intent); not when continuing an already-approved plan (run `specify plan execute`).
argument-hint: <name> [source]...
---

# Plan Skill

The workflow guest owns the whole authoring flow — plan scaffold, per-source survey into `discovery.md`, lead reconciliation into `slices[]`, Gate 1 review prose, validation, and the exit at `pending`. This skill only elicits arguments, invokes the verb, and relays its output.

## Invocation

```bash
specify plan author <name> --source <key>=<adapter>:<binding>
```

- `<name>` is the kebab-case change name (the CLI rejects malformed names).
- Forward each operator-supplied `source <key>=<adapter>:<binding>` positional as one repeated `--source` flag. `<binding>` is a path (`documentation:./design-notes/identity`) or the literal form `value:<literal>` (`intent:value:fix typo in user.rs`).
- When the operator passes no source tokens, elicit a one-line intent and pass it as `--intent "<one-line intent>"` instead of `--source`.

## Relay

- Surface the CLI output verbatim, including the closing Gate 1 hint (the literal `specify plan transition <name> approved` command). Never run that transition yourself — Gate 1 is operator-only.
- On non-zero exit, surface the structured error verbatim and stop. Never hand-edit `plan.yaml`, `change.md`, or `discovery.md` — see [shared guardrails](../../references/guardrails.md#single-writer-for-lifecycle-state).
- Headless Gate 1 curation stays on the CLI: `specify plan add`, `specify plan amend`, `specify plan remove`.
- Workspace plans cannot run under `specify plan execute` (`plan-execute-workspace-unsupported`); drive them hand-driven instead — `specify plan next`, then the `/spec:refine` → `/spec:build` → `/spec:merge` breakouts per slice.

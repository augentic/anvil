---
name: specify-init
description: Initialize Specify in a project by invoking `specify init` and relaying its output. Use when first wiring up a project before any other `/spec:*` command — first-run init, `--upgrade` re-entry, or a registry-only workspace via `/spec:init workspace`.
argument-hint: <adapter|workspace>
---

# Init Skill

`specify init` owns every filesystem write — `.specify/`, `project.yaml`, the adapter cache, root `AGENTS.md`, `.specify/context.lock`, and (for workspaces) the chained initial `workspace sync`. This skill only verifies the binary, elicits arguments, invokes the verb, and relays its output.

## Invocation

1. **Verify the CLI** — run `specify --version`; install with `cargo install --git https://github.com/augentic/specify` only after explicit operator confirmation. Optionally probe drift with `specify upgrade --dry-run` and `specify plugins doctor`, acting (`specify upgrade --yes` / `specify plugins refresh --yes`) only on operator consent; after a plugin refresh, stop for a Cursor restart.
2. **Route re-entry** — when `.specify/project.yaml` already exists, ask before reinitializing and route through `specify init --upgrade`.
3. **Invoke** — `<adapter>` and `--workspace` are mutually exclusive; the literal argument `workspace` means workspace init. When the target adapter declares `platforms.required` (e.g. vectis), elicit the platform set first (the CLI's error names the allowed and default sets; `core` is mandatory):

```bash
specify init <adapter> --platforms <platforms>
```

or `specify init --workspace` for a registry-only workspace.

## Relay

- Surface the CLI output verbatim, including the workspace-sync message and next actions.
- On non-zero exit, surface the structured error and stop — never hand-roll scaffold files, never overwrite `project.yaml` without confirmation, and never pre-populate the out-of-tree adapter cache.

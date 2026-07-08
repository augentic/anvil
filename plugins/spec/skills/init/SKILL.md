---
name: specify-init
description: Initialize Specify in a project by invoking `specify init` and relaying its output. Use when first wiring up a project before any other `/spec:*` command — first-run init, `--upgrade` re-entry, or a registry-only workspace via `/spec:init workspace`.
argument-hint: <adapter|workspace>
---

# Init Skill

`specify init` owns every filesystem write — `.specify/`, `project.yaml`, the global adapter store and project component cache, the generated deployment manifest, root `AGENTS.md`, `.specify/context.lock`, and (for workspaces) the chained initial `workspace sync`. This skill only verifies the binary, elicits arguments, invokes the verb, and relays its output.

## Invocation

1. **Verify the CLI** — run `specify --version`; install with `cargo install --git https://github.com/augentic/specify` only after explicit operator confirmation. Optionally probe drift with `specify upgrade --dry-run` and `specify plugins doctor`, acting (`specify upgrade --yes` / `specify plugins refresh --yes`) only on operator consent; after a plugin refresh, stop for a Cursor restart.
2. **Route re-entry** — when `.specify/project.yaml` already exists, `specify init` changes nothing: it exits 0 and prints the literal `specify init --upgrade` re-entry command. Confirm with the operator, then run `specify init --upgrade`.
3. **Elicit every required input and pass it as a flag** — never rely on the CLI's interactive prompt mode: it engages only on a TTY, and agent shells are not TTYs, so a missing input fails typed instead of prompting (`init-adapter-required` for the adapter; `project-platforms-required` when the target demands `--platforms`, naming the allowed and default sets — `core` is mandatory). Gather conversationally: the adapter (`<adapter>` and `--workspace` are mutually exclusive; the literal argument `workspace` means workspace init), `--platforms <platforms>` when the target adapter declares `platforms.required` (e.g. vectis), and optionally `--name <name>` / `--description "<description>"`.
4. **Invoke**:

```bash
specify init <adapter> [--name <name>] [--description "<description>"] [--platforms <platforms>]
```

or `specify init --workspace` for a registry-only workspace.

## Relay

- Surface the CLI output verbatim — the postflight report names what was scaffolded, the hydrated adapters (`name@version`), the adapter-store root, and the literal next command.
- On non-zero exit, surface the structured error and stop — never hand-roll scaffold files, never overwrite `project.yaml` without confirmation, and never pre-populate the global adapter store or the project component cache by hand.

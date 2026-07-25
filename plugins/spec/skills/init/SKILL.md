---
name: specify-init
description: Initialize Specify in a project by invoking `specify init` and relaying its output. Use when first wiring up a project before any other `/spec:*` command — first-run init, `--upgrade` re-entry, or a registry-only workspace via `/spec:init workspace`.
argument-hint: <adapter|workspace>
---

# Init Skill

`specify init` owns every filesystem write — `.specify/`, `project.yaml`, the project component cache, root `AGENTS.md`, and `.specify/context.lock`. Workspace slot materialization remains operator-owned. This skill installs or refreshes the CLI, elicits arguments, invokes the verb, and relays its output.

## Invocation

1. **Install or refresh the CLI** — invoking this skill is consent to install. Install the prebuilt release via [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) (overwrites any existing binary; no local compile):

```bash
cargo binstall --git https://github.com/augentic/specify specify@0.29.0 --force -y
```

Then run `specify --version` and stop on failure.

2. **Route re-entry** — when `.specify/project.yaml` already exists, `specify init` changes nothing: it exits 0 and prints the literal `specify init --upgrade` re-entry command. Confirm with the operator, then run `specify init --upgrade`.
3. **Elicit every required input and pass it as a flag** — the CLI has no interactive prompt mode: a missing input fails typed (`init-adapter-required` for the adapter; `project-platforms-required` when the target demands `--platforms`, naming the allowed and default sets — `core` is mandatory). Gather conversationally: the adapter (`<adapter>` and `--workspace` are mutually exclusive; the literal argument `workspace` means workspace init), `--platforms <platforms>` when the target adapter declares `platforms.required` (e.g. vectis), and optionally `--name <name>` / `--description "<description>"`.
4. **Invoke**:

```bash
specify init <adapter> [--name <name>] [--description "<description>"] [--platforms <platforms>]
```

or `specify init --workspace` for a registry-only workspace.

## Relay

- Surface the CLI output verbatim — the postflight report names what was scaffolded and the literal next command.
- On non-zero exit, surface the structured error and stop — never hand-roll scaffold files, never overwrite `project.yaml` without confirmation, and never pre-populate the project component cache by hand.

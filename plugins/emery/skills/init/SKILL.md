---
name: emery-init
description: Initialize Emery in a project by invoking `emery init` and relaying its output. Use when first wiring up a project before any other `/emery:*` command — first-run init, `--upgrade` re-entry, or a registry-only workspace via `/emery:init workspace`.
argument-hint: <adapter|workspace>
---

# Init Skill

`emery init` owns every filesystem write — `.emery/`, `project.yaml`, the project component cache, root `AGENTS.md`, and `.emery/context.lock`. Workspace slot materialization remains operator-owned. This skill installs or refreshes the CLI, elicits arguments, invokes the verb, and relays its output.

## Invocation

1. **Install or refresh the CLI** — invoking this skill is consent to install. Install the prebuilt release via the installer script (overwrites any existing binary; no local compile; verifies the Release archive's `.sha256`). The script installs to `~/.local/bin`, which is often absent from `PATH`, so put it on the session's `PATH` first — the subprocess cannot alter the parent shell:

```bash
export PATH="$HOME/.local/bin:$PATH"
curl -fsSL https://raw.githubusercontent.com/augentic/emery/main/scripts/install.sh | sh -s -- --version 0.32.0 -y
```

Then run `RUST_LOG=off emery --version` and stop on failure. Run every subsequent `emery` command in this session with that `PATH` export in effect; remind the operator to add `export PATH="$HOME/.local/bin:$PATH"` to their shell profile if the installer printed a PATH note.

2. **Route re-entry** — when `.emery/project.yaml` already exists, `emery init` changes nothing: it exits 0 and prints the literal `emery init --upgrade` re-entry command. Confirm with the operator, then run `RUST_LOG=off emery init --upgrade`.
3. **Elicit every required input and pass it as a flag** — the CLI has no interactive prompt mode: a missing input fails typed (`init-adapter-required` for the adapter; `project-platforms-required` when the target demands `--platforms`, naming the allowed and default sets — `core` is mandatory). Gather conversationally: the adapter (`<adapter>` and `--workspace` are mutually exclusive; the literal argument `workspace` means workspace init), `--platforms <platforms>` when the target adapter declares `platforms.required` (e.g. vectis), and optionally `--name <name>` / `--description "<description>"`.
4. **Invoke**:

```bash
RUST_LOG=off emery init <adapter> [--name <name>] [--description "<description>"] [--platforms <platforms>]
```

or `RUST_LOG=off emery init --workspace` for a registry-only workspace. Init is a short deterministic verb — it runs quiet per the plugin rule's *Tracing and output* contract (the debug variant applies when the operator asks for debug).

## Relay

- Surface the CLI output verbatim — the postflight report names what was scaffolded and the literal next command.
- On non-zero exit, surface the structured error and stop — never hand-roll scaffold files, never overwrite `project.yaml` without confirmation, and never pre-populate the project component cache by hand.

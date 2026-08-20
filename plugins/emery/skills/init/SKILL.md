---
name: emery-init
description: Initialize Emery in a project by invoking `emery init` and relaying its output. Use when first wiring up a project before any other `/emery:*` command — first-run init or `--upgrade` re-entry.
argument-hint: <adapter>
---

# Init Skill

`emery init` owns every filesystem write — `.emery/`, `project.yaml`, and the project component cache. This skill installs or refreshes the CLI, elicits arguments, invokes the verb, and relays its output.

## Invocation

1. **Install or refresh the CLI** — on a machine with no `emery` binary, invoking this skill is consent to install. When `emery` is already on `PATH`, confirm with the operator before reinstalling. Install the latest prebuilt release via Homebrew, or from source; a project whose floor outruns the installed binary fails typed later (`emery-version-too-old`, exit 3) with the same reinstall command as its hint:

```bash
brew tap augentic/tap
brew install emery
# or: cargo install --git https://github.com/augentic/emery --locked
```

Then run `emery --version --quiet` and stop on failure.

2. **Route re-entry** — when `.emery/project.yaml` already exists, `emery init` changes nothing: it exits 0 and prints the literal `emery init --upgrade` re-entry command. Confirm with the operator, then run `emery init --upgrade --quiet`.
3. **Elicit every required input and pass it as a flag** — the CLI has no interactive prompt mode: no source at all fails typed (`init-source-required`). Gather conversationally: the source adapters to bind (each positional `<adapter>` is a workspace-backed source; each `--value <adapter>=<text>` is an inline source such as an operator directive), and optionally `--name <name>` / `--description "<description>"`.
4. **Invoke**:

```bash
emery init <adapter>... [--value <adapter>=<text>] [--name <name>] [--description "<description>"] --quiet
```

Init is a short deterministic verb — it runs with `--quiet` per the plugin rule's *Tracing and output* contract (`--debug` replaces it when the operator asks for debug).

## Relay

- Surface the CLI output verbatim — the postflight report names what was scaffolded and the literal next command.
- On non-zero exit, surface the structured error and stop — never hand-roll scaffold files, never overwrite `project.yaml` without confirmation, and never pre-populate the project component cache by hand.

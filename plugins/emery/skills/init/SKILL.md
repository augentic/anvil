---
name: emery-init
description: Initialize Emery in a project by invoking `emery init` and relaying its output. Use when first wiring up a project before any other `/emery:*` command — first-run init or `--upgrade` re-entry.
argument-hint: <adapter>
---

# Init Skill

`emery init` owns every filesystem write — `.emery/`, `project.yaml`, the project component cache, root `AGENTS.md`, and `.emery/context.lock`. This skill installs or refreshes the CLI, elicits arguments, invokes the verb, and relays its output.

## Invocation

1. **Install or refresh the CLI** — on a machine with no `emery` binary, invoking this skill is consent to install. When `emery` is already on `PATH`, confirm with the operator before reinstalling — the installer overwrites the existing binary. Install the latest prebuilt release via the installer script (no local compile; verifies the Release archive's `.sha256`); a project whose floor outruns the installed binary fails typed later (`emery-version-too-old`, exit 3) with the same reinstall command as its hint. The script installs to `~/.local/bin`, which is often absent from `PATH`, so put it on the session's `PATH` first — the subprocess cannot alter the parent shell:

```bash
export PATH="$HOME/.local/bin:$PATH"
curl -fsSL https://raw.githubusercontent.com/augentic/emery/main/scripts/install.sh | sh -s -- -y
```

Then run `emery --version --quiet` and stop on failure. Run every subsequent `emery` command in this session with that `PATH` export in effect; remind the operator to add `export PATH="$HOME/.local/bin:$PATH"` to their shell profile if the installer printed a PATH note.

2. **Route re-entry** — when `.emery/project.yaml` already exists, `emery init` changes nothing: it exits 0 and prints the literal `emery init --upgrade` re-entry command. Confirm with the operator, then run `emery init --upgrade [--gap-policy <strict|defer>] --quiet` — an existing `gap-policy:` declaration is preserved when the flag is absent and updated when passed.
3. **Elicit every required input and pass it as a flag** — the CLI has no interactive prompt mode: a missing input fails typed (`init-adapter-required` for the adapter; `project-platforms-required` when the target demands `--platforms`, naming the allowed and default sets — `core` is mandatory). Gather conversationally: the adapter, `--platforms <platforms>` when the target adapter declares `platforms.required` (e.g. vectis), and optionally `--name <name>` / `--description "<description>"` / `--gap-policy <strict|defer>` (the standing gap policy recorded on `project.yaml`; absent means `strict`).
4. **Invoke**:

```bash
emery init <adapter> [--name <name>] [--description "<description>"] [--platforms <platforms>] [--gap-policy <strict|defer>] --quiet
```

Init is a short deterministic verb — it runs with `--quiet` per the plugin rule's *Tracing and output* contract (`--debug` replaces it when the operator asks for debug).

## Relay

- Surface the CLI output verbatim — the postflight report names what was scaffolded and the literal next command.
- On non-zero exit, surface the structured error and stop — never hand-roll scaffold files, never overwrite `project.yaml` without confirmation, and never pre-populate the project component cache by hand.

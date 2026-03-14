---
name: init
description: Initialize Specify in a project. Creates the .specify/ directory structure and config.yaml. Use when setting up a new project for spec-driven development.
license: MIT
argument-hint: [schema?]
allowed-tools: Read, Write, Shell, Grep, WebFetch
---

## Arguments

```text
$SCHEMA         = $ARGUMENTS[0]
```

I'll create the `.specify/` directory structure and install a starter `config.yaml` for you to customize.

---

**Input**: None required. Optionally a schema (name or URL) and project context.

**Steps**

1. **Check if already initialized**

   Check whether `.specify/config.yaml` exists.

   - If it exists, inform the user: "Specify is already initialized in this project. Your config is at `.specify/config.yaml`."
   - Use **AskQuestion tool** to confirm whether they want to reinitialize (which overwrites config).
   - If they decline, stop.

2. **Resolve schema**

   If `$SCHEMA` is provided (as an argument), use it directly. Otherwise, use the **AskQuestion tool** to let the user select:
   - **omnia**: New development (JIRA -> Rust WASM)
   - **realtime**: Migration (TypeScript -> Rust WASM)

   Store the result as `$SCHEMA`.

   Resolve `$SCHEMA` using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `config.yaml`.

3. **Create directory structure**

   ```bash
   mkdir -p .specify/changes .specify/specs .specify/.cache
   ```

   If `.specify/.gitignore` does not exist, create it with:
   ```
   .cache/
   ```

4. **Install config.yaml**

   Read `config.yaml` from the resolved schema directory and write it to `.specify/config.yaml`.

   If the user provided project context, fill in the `context` field with their description. Otherwise keep the schema's default context.

   If schema resolution failed (no matching directory, fetch error), warn the user and stop — a valid schema is required.

5. **Prompt for customization**

   Tell the user:
   - "Specify initialized. Config written to `.specify/config.yaml`."
   - "Edit the `context` field to describe your project's tech stack, architecture, and testing approach."
   - "The `rules` section contains the schema defaults. Edit any artifact key under `rules` to override its defaults. Keys you don't change will keep the schema defaults automatically."
   - "When ready, run `/spec:propose` to start your first change."

**Output**

```
## Specify Initialized

**Schema**: $SCHEMA
**Config**: .specify/config.yaml
**Changes**: .specify/changes/
**Baseline specs**: .specify/specs/

Next steps:
1. Edit `.specify/config.yaml` to describe your project
2. Run `/spec:propose` to create your first change
```

**Guardrails**
- Do not overwrite an existing config without user confirmation
- The schema's `config.yaml` is the source of truth for default config content — do not use an inline template
- Do not create `.specify/schemas/` — schemas are resolved from the plugin directory or fetched from URLs
- If schema resolution fails, stop and report the error rather than creating a config with unknown schema content

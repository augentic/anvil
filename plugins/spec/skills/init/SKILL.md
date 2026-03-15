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

   Resolve `$SCHEMA` using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`, `config.yaml`, `instructions/*`.

3. **Create directory structure**

   ```bash
   mkdir -p .specify/changes .specify/specs .specify/.cache
   ```

   If `.specify/.gitignore` does not exist, create it with:
   ```
   .cache/
   ```

4. **Populate schema cache**

   Copy all resolved schema files into `.specify/.cache/`, mirroring the schema directory structure:

   ```text
   .specify/.cache/
   ├── .cache-meta.yaml
   ├── schema.yaml
   ├── config.yaml
   └── instructions/
       ├── proposal.md
       ├── specs.md
       ├── design.md
       ├── tasks.md
       └── apply.md
   ```

   Write `.specify/.cache/.cache-meta.yaml` with:
   - `schema_url`: the full `$SCHEMA` value. For bare-name schemas (no `/`), use `local:<name>` (e.g., `local:omnia`). For URL-based schemas, use the full URL (including `@ref` if present).
   - `fetched_at`: current ISO-8601 timestamp

   If the resolved schema directory contains an `instructions/` subdirectory, create `.specify/.cache/instructions/` and copy all files from it.

5. **Install config.yaml**

   Write a thin project config to `.specify/config.yaml` with:
   - `schema`: set to `$SCHEMA` (the resolved schema value — bare name or URL)
   - `context`: set to the user's description if provided, otherwise a placeholder comment (`# Describe your project here`)
   - `rules`: scaffold one key per artifact defined in the resolved `schema.yaml` (read `artifacts[].id`). Each key is a YAML block scalar (`|`) containing a placeholder comment. For example, with the omnia schema the output is:

     ```yaml
     rules:
       proposal: |
         # TODO: Add any proposal override rules here
       specs: |
         # TODO: Add any specs override rules here
       design: |
         # TODO: Add any design override rules here
       tasks: |
         # TODO: Add any tasks override rules here
     ```

     These are overrides only — schema defaults from `.specify/.cache/config.yaml` still apply for any key left as a placeholder.

   Do NOT copy the schema's `config.yaml` wholesale. The project config is a thin overlay; schema defaults live in the cache.

   If schema resolution failed (no matching directory, fetch error), warn the user and stop — a valid schema is required.

6. **Prompt for customization**

   Tell the user:
   - "Specify initialized. Config written to `.specify/config.yaml`."
   - "Edit the `context` field to describe your project's tech stack, architecture, and testing approach."
   - "Fill in the scaffolded `rules` entries to override schema defaults for specific artifacts. To see the defaults, check `.specify/.cache/config.yaml`."
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
- Write a thin project config with `schema`, `context`, and scaffolded `rules` keys (one per schema artifact) — schema defaults live in `.specify/.cache/config.yaml`
- Populate `.specify/.cache/` with the full schema so downstream skills resolve from cache
- If schema resolution fails, stop and report the error rather than creating a config with unknown schema content

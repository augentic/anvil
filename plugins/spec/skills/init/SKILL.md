---
name: init
description: Initialize Specify in a project. Creates the .specify/ directory structure and project.yaml. Use when setting up a new project for spec-driven development.
license: MIT
argument-hint: "[schema?]"
allowed-tools: Read, Write, Shell, Grep, WebFetch
---

## Arguments

```text
$SCHEMA         = $ARGUMENTS[0]
```

I'll create the `.specify/` directory structure and install a starter `project.yaml` for you to customize.

---

**Input**: None required. Optionally a schema (name or URL) and project context.

**Steps**

1. **Check if already initialized**

   Check whether `.specify/project.yaml` exists.

   - If it exists, inform the user: "Specify is already initialized in this project. Your config is at `.specify/project.yaml`."
   - Use **AskQuestion tool** to confirm whether they want to reinitialize (which overwrites project.yaml).
   - If they decline, stop.

2. **Resolve schema**

   If `$SCHEMA` is provided (as an argument), use it directly. Otherwise, list available schemas from the `schemas/` directory (each subdirectory containing a `schema.yaml` is a schema). If only one schema exists, use it as the default and confirm with the user. If multiple schemas exist, use the **AskQuestion tool** to let the user select from the available options.

   Store the result as `$SCHEMA`.

   Resolve `$SCHEMA` using the **Schema Resolution** procedure (`references/schema-resolution.md`). Files needed: `schema.yaml`, `briefs/*`.

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
   └── briefs/
       ├── proposal.md
       ├── specs.md
       ├── design.md
       ├── tasks.md
       ├── build.md
       └── merge.md
   ```

   Write `.specify/.cache/.cache-meta.yaml` with:
   - `schema_url`: the full `$SCHEMA` value. For bare-name schemas (no `/`), use `local:<name>` (e.g., `local:omnia`). For URL-based schemas, use the full URL (including `@ref` if present).
   - `fetched_at`: current ISO-8601 timestamp

   If the resolved schema directory contains a `briefs/` subdirectory, create `.specify/.cache/briefs/` and copy all files from it.

5. **Install project.yaml**

   Write a thin project config to `.specify/project.yaml` with:
   - `name`: set to the project directory name (or the user's provided name)
   - `domain`: set to the user's description if provided, otherwise a placeholder comment (`# Describe your project here`)
   - `schema`: set to `$SCHEMA` (the resolved schema value — bare name or URL)
   - `rules`: scaffold one key per brief defined in `pipeline.define` of the resolved `schema.yaml` (read each entry's `id`). Each key is an empty string (no override). Add a comment showing the file-path format so the user knows how to add rules later. For example, with the omnia schema the output is:

     ```yaml
     name: my-project
     domain: |
       # Describe your project here
     schema: omnia

     rules:
       # proposal:  # e.g. rules/proposal.md
       # specs:
       # design:
       # tasks: 
     ```

     Each value is a relative file path (from `.specify/`) to a markdown file containing additional rules for that brief. An empty string means no override — the schema brief's body text is used as-is. The schema `domain` in `.specify/.cache/schema.yaml` provides fallback context.

   Do NOT copy the schema's domain wholesale. The project config is a thin overlay; the schema domain lives in `schema.yaml`.

   If schema resolution failed (no matching directory, fetch error), warn the user and stop — a valid schema is required.

6. **Prompt for customization**

   Tell the user:
   - "Specify initialized. Config written to `.specify/project.yaml`."
   - "Edit the `domain` field to describe your project's tech stack, architecture, and testing approach."
   - "Fill in the scaffolded `rules` entries to add project-level rules for specific artifacts. For fallback context, check the `domain` section in `.specify/.cache/schema.yaml`."

   Do NOT print "Next steps" yet — Step 7 determines which output to show.

7. **Detect existing codebase and offer baseline extraction**

   Check whether the project root contains an active codebase by looking for:

   - **Manifest files**: `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `pom.xml`, `*.csproj`, `build.gradle`, `Gemfile`
   - **Source directories**: `src/`, `lib/`, `app/`, `cmd/`

   If **none** of these are found, show the **greenfield output** and stop.

   If at least one indicator is found, use the **AskQuestion tool**:

   > "I've detected an existing codebase (found `<indicator>`). Would you like me to analyze it and generate baseline specs that capture its current behavior? This uses `/spec:extract` and typically takes a few minutes with your input at checkpoints."

   Options:
   - **Yes, generate baseline specs** — proceed to create the change
   - **No, skip for now** — show the greenfield output and stop (user can run `/spec:extract` manually later)

   If the user chooses **yes**:

   a. Create the change directory and metadata:

      ```bash
      mkdir -p .specify/changes/initial-baseline/specs
      ```

      Write `.specify/changes/initial-baseline/.metadata.yaml`:

      ```yaml
      schema: $SCHEMA
      status: defining
      created_at: <current ISO-8601 timestamp>
      defined_at: null
      build_started_at: null
      completed_at: null
      touched_specs: []
      ```

   b. Show the **brownfield output** and stop.

**Output (greenfield — no existing codebase, or user declined extraction)**

```
## Specify Initialized

**Schema**: $SCHEMA
**Config**: .specify/project.yaml
**Changes**: .specify/changes/
**Baseline specs**: .specify/specs/

Next steps:
1. Edit `.specify/project.yaml` to describe your project
2. Run `/spec:define` to create your first change
```

**Output (brownfield — user opted for baseline extraction)**

```
## Specify Initialized (Existing Codebase Detected)

**Schema**: $SCHEMA
**Config**: .specify/project.yaml
**Baseline change**: .specify/changes/initial-baseline/

Next steps:
1. Edit `.specify/project.yaml` to describe your project
2. Run `/spec:extract . .specify/changes/initial-baseline/` to analyze the codebase
3. After extraction, run `/spec:merge initial-baseline` to promote specs to baseline
4. Then run `/spec:define` for future changes
```

**Guardrails**
- Do not overwrite an existing project.yaml without user confirmation
- Write a thin project config with `name`, `domain`, `schema`, and scaffolded `rules` keys (one per `pipeline.define` entry) — the schema `domain` in `schema.yaml` provides fallback context
- Populate `.specify/.cache/` with the full schema so downstream skills resolve from cache
- If schema resolution fails, stop and report the error rather than creating a project.yaml with unknown schema content

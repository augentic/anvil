---
name: init
description: Initialize Specify in a project. Populates `.specify/.cache/` and invokes `specify init` to scaffold `.specify/` and write `project.yaml`. Use when setting up a new project for spec-driven development.
license: MIT
argument-hint: "[schema?]"
allowed-tools: Read, Write, Shell, Grep, WebFetch, AskQuestion
---

## Prerequisites

**If `specify` is not on PATH:** stop and instruct the user to install the
CLI via `brew install specify` (preferred), `cargo install specify`, or
the release script at https://specify.sh/install, then re-run. Do not
attempt a prose fallback — validation rules have diverged past the point
where the agent can reliably reproduce them.

## Arguments

```text
$SCHEMA         = $ARGUMENTS[0]
```

I'll populate `.specify/.cache/` with the schema and invoke `specify init` to install a starter `project.yaml`.

---

**Input**: None required. Optionally a schema (name or URL) and project context.

**Steps**

1. **Check if already initialized**

   Check whether `.specify/project.yaml` exists.

   - If it exists, inform the user: "Specify is already initialized in this project. Your config is at `.specify/project.yaml`."
   - Use **AskQuestion tool** to confirm whether they want to reinitialize (which overwrites project.yaml).
   - If they decline, stop.
   - If they confirm, treat the run as `$UPGRADE=true` so the CLI rewrites `specify_version` to the running binary.

2. **Resolve schema**

   If `$SCHEMA` is provided (as an argument), use it directly. Otherwise, list available schemas from the `schemas/` directory (each subdirectory containing a `schema.yaml` is a schema). If only one schema exists, use it as the default and confirm with the user. If multiple schemas exist, use the **AskQuestion tool** to let the user select from the available options.

   Store the result as `$SCHEMA`.

3. **Populate the schema cache**

   Per RFC-1 §`schema.rs` the agent owns all writes to `.specify/.cache/`. The CLI reads the cache but never fetches. Before invoking `specify init`, mirror the resolved schema tree under `.specify/.cache/<name>/` so the CLI can resolve it:

   ```text
   .specify/.cache/
   ├── .cache-meta.yaml
   └── <schema-name>/
       ├── schema.yaml
       └── briefs/
           ├── proposal.md
           ├── specs.md
           ├── design.md
           ├── tasks.md
           ├── build.md
           └── merge.md
   ```

   Use the **Schema Resolution** procedure (`references/schema-resolution.md`) to locate schema files. Files needed: `schema.yaml` and every file referenced by `pipeline.{define,build,merge}[].brief`. For URL-based schemas, fetch them with **WebFetch**; for bare-name schemas, copy from the local `schemas/<name>/` tree.

   Write `.specify/.cache/.cache-meta.yaml` with:
   - `schema_url`: the full `$SCHEMA` value. For bare-name schemas (no `/`), use `local:<name>` (e.g. `local:omnia`). For URL-based schemas, use the full URL (including `@ref` if present).
   - `fetched_at`: current ISO-8601 timestamp.

   If schema resolution or fetch fails, warn the user and stop — a valid schema is required before invoking the CLI.

4. **Collect project metadata**

   Determine `$PROJECT_NAME` (default: project directory basename) and optionally `$DOMAIN` (project description). Use the **AskQuestion tool** to confirm `$PROJECT_NAME` and to prompt for `$DOMAIN` if the user hasn't supplied one. An empty `$DOMAIN` is fine — the CLI omits the field.

5. **Invoke `specify init`**

   ```bash
   specify init "$SCHEMA" \
     --schema-dir . \
     --name "$PROJECT_NAME" \
     ${DOMAIN:+--domain "$DOMAIN"} \
     ${UPGRADE:+--upgrade} \
     --format json
   ```

   The CLI creates `.specify/{changes,specs,archive,.cache}/`, writes `.specify/project.yaml` with one empty `rules:` entry per `pipeline.define` brief, upserts `.specify/.cache/` into the project `.gitignore`, and records `specify_version`. Parse the JSON response to capture `config_path`, `schema_name`, `cache_present`, `directories_created`, `scaffolded_rule_keys`, and `specify_version` for reporting back to the user.

   On non-zero exit, surface the JSON `error`/`message` fields. Do not attempt a prose fallback.

6. **Prompt for customization**

   Tell the user:
   - "Specify initialized. Config written to `.specify/project.yaml`."
   - "Edit the `domain` field to describe your project's tech stack, architecture, and testing approach."
   - "Fill in the scaffolded `rules` entries to add project-level rules for specific artifacts. For fallback context, check the `domain` section in `.specify/.cache/<schema>/schema.yaml`."

   Do NOT print "Next steps" yet — Step 7 determines which output to show.

7. **Detect existing codebase and offer baseline extraction**

   Check whether the project root contains an active codebase by looking for:

   - **Manifest files**: `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `pom.xml`, `*.csproj`, `build.gradle`, `Gemfile`
   - **Source directories**: `src/`, `lib/`, `app/`, `cmd/`

   If **none** of these are found, show the **greenfield output** and stop.

   If at least one indicator is found, use the **AskQuestion tool**:

   > "I've detected an existing codebase (found `<indicator>`). Would you like me to analyze it and generate baseline specs that capture its current behavior? This uses `/spec:extract`."

   Options:
   - **Yes, generate baseline specs** — proceed to create the change
   - **No, skip for now** — show the greenfield output and stop (user can run `/spec:extract` manually later)

   If the user chooses **yes**, create the change via the CLI:

   ```bash
   specify change create initial-baseline --format json
   ```

   The CLI validates the name, creates `.specify/changes/initial-baseline/specs/`, and writes the initial `.metadata.yaml` (status `defining`, `created_at` timestamp). Show the **brownfield output** and stop.

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
- Populate `.specify/.cache/` before invoking `specify init` — the agent owns cache writes; the CLI only reads
- If the CLI exits non-zero, surface the error and stop; do not hand-roll the scaffold

> Implements RFC-1 Phase 1 — the CLI handles deterministic operations.

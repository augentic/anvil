---
name: init
description: Initialize Specify in a project. Decides between a regular single-project init and a registry-only platform hub (RFC-9 §1D). Populates `.specify/.cache/` (regular only) and invokes `specify init` (with `--hub` for hubs) to scaffold `.specify/` and write `project.yaml`. Use when setting up a new project for spec-driven development.
license: MIT
argument-hint: "schema?"
allowed-tools: Read Write Shell Grep WebFetch AskQuestion
---

## Prerequisites

**If `specify` is not on PATH:** stop and instruct the user to install the CLI via `brew install augentic/tap/specify` (preferred), `cargo install specify`, or the release script at https://specify.sh/install, then re-run. Do not attempt a prose fallback — validation rules have diverged past the point where the agent can reliably reproduce them.

## Arguments

```text
$SCHEMA         = $ARGUMENTS[0]
```

I'll decide whether this is a regular single-project init or a registry-only platform hub, populate `.specify/.cache/` if needed, and invoke `specify init` (with `--hub` for hubs) to install a starter `project.yaml`.

---

**Input**: None required. Optionally a schema (name or URL) and project context. The schema argument is irrelevant for hub mode and is ignored when `--hub` is set.

**Steps**

1. **Check if already initialized**

   Check whether `.specify/project.yaml` exists.

   - If it exists, inform the user: "Specify is already initialized in this project. Your config is at `.specify/project.yaml`."
   - Use **AskQuestion tool** to confirm whether they want to reinitialize (which overwrites project.yaml).
   - If they decline, stop.
   - If they confirm, treat the run as `$UPGRADE=true` so the CLI rewrites `specify-version` to the running binary.

2. **Decide the topology — regular project or platform hub**

   See [Platform repo topologies](../../../../docs/explanation/platform-repo.md) for the full background on the two shapes. Briefly:

   - **Regular project** — a single repository that contains both code and `.specify/`. The most common shape; choose this for single-repo projects, small teams, and any case where the operator just wants to track changes against the code in this repo. Phase pipelines (define / build / merge) run against this repo's working tree.
   - **Platform hub** (RFC-9 §1D) — a registry-only repository that holds platform state (`registry.yaml`, `initiative.md`, `plan.yaml`, `workspace/`) but never carries code itself. Choose this when the platform spans multiple repos and the operator wants the platform repo's identity to be unambiguous. Phase pipelines are disabled on the hub itself; code lives in registered project repos under `.specify/workspace/<name>/`.

   Ask the user via **AskQuestion tool** unless the answer is obvious from context (e.g. an existing `Cargo.toml` / `package.json` / `src/` strongly implies a regular project, while an empty directory in a multi-repo organisation often points at a hub). Treat the result as `$HUB_MODE=true|false`.

   Branch:

   - When `$HUB_MODE=true`, skip steps 3 and 4's schema-resolution work and jump to step 5's hub invocation.
   - When `$HUB_MODE=false`, continue with the schema-driven flow below.

3. **Resolve schema** *(regular only — skip in hub mode)*

   If `$SCHEMA` is provided (as an argument), use it directly. Otherwise, list available schemas from the `schemas/` directory (each subdirectory containing a `schema.yaml` is a schema). If only one schema exists, use it as the default and confirm with the user. If multiple schemas exist, use the **AskQuestion tool** to let the user select from the available options.

   Store the result as `$SCHEMA`.

4. **Populate the schema cache** *(regular only — skip in hub mode)*

   The agent owns all writes to `.specify/.cache/`. The CLI reads the cache but never fetches. Before invoking `specify init`, mirror the resolved schema tree under `.specify/.cache/<name>/` so the CLI can resolve it:

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

5. **Collect project metadata and invoke `specify init`**

   Determine `$PROJECT_NAME` (default: project directory basename) and optionally `$DOMAIN` (project description). Use the **AskQuestion tool** to confirm `$PROJECT_NAME` and to prompt for `$DOMAIN` if the user hasn't supplied one. An empty `$DOMAIN` is fine — the CLI omits the field. For hub mode, `$PROJECT_NAME` MUST be kebab-case (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens) — the CLI bakes it into `.specify/initiative.md`'s frontmatter and rejects non-kebab values.

   **Regular invocation:**

   ```bash
   specify init "$SCHEMA" \
     --schema-dir . \
     --name "$PROJECT_NAME" \
     ${DOMAIN:+--domain "$DOMAIN"} \
     ${UPGRADE:+--upgrade} \
     --format json
   ```

   **Hub invocation** (when `$HUB_MODE=true`):

   ```bash
   specify init hub \
     --schema-dir . \
     --name "$PROJECT_NAME" \
     ${DOMAIN:+--domain "$DOMAIN"} \
     --hub \
     --format json
   ```

   The first positional argument (`hub`) is the `schema` value — ignored in hub mode but still required by the CLI parser. Any string works; `hub` is a convenient placeholder. The CLI writes:

   - **Regular** — `.specify/{changes,specs,archive,.cache}/`, `.specify/project.yaml` with one empty `rules:` entry per `pipeline.define` brief, `.specify/.cache/` upserted into `.gitignore`, and `specify-version` recorded.
   - **Hub** — `.specify/project.yaml` with `schema: hub`, `hub: true`, no `rules:` block; `.specify/registry.yaml` with `version: 1` and `projects: []`; `.specify/initiative.md` from the canonical template named after `$PROJECT_NAME`; `.specify/.cache/` and `.specify/workspace/` upserted into `.gitignore`. Phase-pipeline directories (`changes/`, `specs/`, `.cache/`) are NOT scaffolded — the hub disables those pipelines.

   In both modes, parse the JSON response to capture `config-path`, `schema-name`, `cache-present`, `directories-created`, `scaffolded-rule-keys`, `specify-version`, and `hub` for reporting back to the user. The `hub` field is `true` only when `--hub` was passed.

   On non-zero exit, surface the JSON `error`/`message` fields. Do not attempt a prose fallback. Hub mode in particular refuses to scaffold over an existing `.specify/` directory — if the user wants to convert an existing single-repo project into a hub, they remove `.specify/` first.

6. **Prompt for customization**

   For a **regular** init, tell the user:
   - "Specify initialized. Config written to `.specify/project.yaml`."
   - "Edit the `domain` field to describe your project's tech stack, architecture, and testing approach."
   - "Fill in the scaffolded `rules` entries to add project-level rules for specific artifacts. For fallback context, check the `domain` section in `.specify/.cache/<schema>/schema.yaml`."

   For a **hub** init, tell the user:
   - "Specify initialized as a registry-only platform hub. Config written to `.specify/project.yaml`."
   - "Add code projects to `.specify/registry.yaml` once they exist. The hub starts with `projects: []`."
   - "Edit `.specify/initiative.md` to frame the first initiative this hub will drive."

   Do NOT print "Next steps" yet — Step 7 determines which output to show.

7. **Detect existing codebase and offer baseline extraction** *(regular only — skip in hub mode)*

   When `$HUB_MODE=true`, skip this step entirely and show the **hub output** below. A hub never carries code, so codebase detection and baseline extraction do not apply.

   For regular projects, check whether the project root contains an active codebase by looking for:

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

**Output (greenfield — regular project, no existing codebase, or user declined extraction)**

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

**Output (brownfield — regular project, user opted for baseline extraction)**

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

**Output (hub — `$HUB_MODE=true`)**

```
## Specify Initialized (Platform Hub)

**Topology**: registry-only hub (RFC-9 §1D)
**Config**: .specify/project.yaml (`schema: hub`, `hub: true`)
**Registry**: .specify/registry.yaml (`version: 1`, `projects: []`)
**Initiative brief**: .specify/initiative.md

Next steps:
1. Add registered projects to `.specify/registry.yaml` (hand-edit, or `specify registry add` once that verb lands)
2. Edit `.specify/initiative.md` to frame the first initiative
3. Run `/spec:plan <name>` to author a plan, then `/spec:execute --loop` to drive it
```

**Guardrails**
- Do not overwrite an existing project.yaml without user confirmation
- For regular projects, populate `.specify/.cache/` before invoking `specify init` — the agent owns cache writes; the CLI only reads
- For hubs, never populate `.specify/.cache/` and never resolve a schema — the `hub` sentinel disables phase pipelines on the hub itself, so there is no schema to cache
- Hub init refuses to run over an existing `.specify/`; if the user wants to convert a regular project into a hub, they must remove `.specify/` first
- If the CLI exits non-zero, surface the error and stop; do not hand-roll the scaffold

---
name: specify-init
description: Initialize Specify in a project. Bootstraps the `specify` CLI when missing, decides between a regular single-project init and a registry-only platform hub, then invokes `specify init` with `--schema-uri` or `--hub` to scaffold `.specify/` and write `project.yaml`. Use when setting up a new project for spec-driven development.
argument-hint: "[schema-url]"
---

## CLI bootstrap

`/spec:init` is the one Specify skill that may install the CLI before continuing. Other CLI-dependent skills still stop when `specify` is missing.

## Arguments

```text
$SCHEMA_URI     = $ARGUMENTS[0]
```

I'll ensure the `specify` CLI is available, decide whether this is a regular single-project init or a registry-only platform hub, then invoke `specify init` (with `--schema-uri` for regular projects or `--hub` for hubs) to install a starter `project.yaml`.

---

**Input**: None required. Optionally a schema URI and project context. The schema argument is irrelevant for hub mode.

**Steps**

1. **Ensure the CLI is available**

   Run:

   ```bash
   specify --version
   ```

   If the command succeeds, continue to step 2.

   If `specify` is not on PATH, tell the user:

   > "The `specify` CLI is required before I can initialize this project. I can install it now with `cargo install --git https://github.com/augentic/specify-cli`, then verify `specify --version` before continuing."

   Use the **AskQuestion tool** to confirm whether they want to install the CLI now.

   - If they decline, stop and tell them to install the CLI manually, then re-run `/spec:init`.
   - If they confirm, run:

     ```bash
     cargo install --git https://github.com/augentic/specify-cli
     ```

   After installation, run `specify --version` again.

   - If verification succeeds, continue.
   - If installation or verification fails, surface the error and stop. Do not attempt a prose fallback or hand-roll `.specify/` scaffolding.

2. **Check if already initialized**

   Check whether `.specify/project.yaml` exists.

   - If it exists, inform the user: "Specify is already initialized in this project. Your config is at `.specify/project.yaml`."
   - Use **AskQuestion tool** to confirm whether they want to reinitialize (which overwrites project.yaml).
   - If they decline, stop.
   - If they confirm, treat the run as `$UPGRADE=true` so the CLI rewrites `specify-version` to the running binary.

3. **Decide the topology — regular project or platform hub**

   See [Platform repo topologies](../../../../docs/explanation/platform-repo.md) for the full background on the two shapes. Briefly:

   - **Regular project** — a single repository that contains both code and `.specify/`. The most common shape; choose this for single-repo projects, small teams, and any case where the operator just wants to track changes against the code in this repo. Phase pipelines (define / build / merge) run against this repo's working tree.
   - **Platform hub** (RFC-9 §1D) — a registry-only repository that holds platform state (`registry.yaml`, `initiative.md`, `plan.yaml`, `workspace/`) but never carries code itself. Choose this when the platform spans multiple repos and the operator wants the platform repo's identity to be unambiguous. Phase pipelines are disabled on the hub itself; code lives in registered project repos under `.specify/workspace/<name>/`.

   Ask the user via **AskQuestion tool** unless the answer is obvious from context (e.g. an existing `Cargo.toml` / `package.json` / `src/` strongly implies a regular project, while an empty directory in a multi-repo organisation often points at a hub). Treat the result as `$HUB_MODE=true|false`.

   Branch:

   - When `$HUB_MODE=true`, skip step 4's schema selection and jump to step 5's hub invocation.
   - When `$HUB_MODE=false`, continue with the schema-driven flow below.

4. **Choose schema URI** *(regular only — skip in hub mode)*

   If `$SCHEMA_URI` is provided (as an argument), use it directly. Otherwise, prefer the canonical Omnia schema URI unless project context clearly indicates another schema:

   ```text
   https://github.com/augentic/specify/schemas/omnia
   ```

   For local development in this repository, a local schema directory such as `./schemas/omnia` is also valid. If multiple schemas are plausible, use the **AskQuestion tool** to let the user select the schema URI.

   Store the result as `$SCHEMA_URI`. Do not pre-populate `.specify/.cache/`; the CLI owns schema fetch/copy during `specify init --schema-uri`.

5. **Collect project metadata and invoke `specify init`**

   Determine `$PROJECT_NAME` (default: project directory basename) and optionally `$DOMAIN` (project description). Use the **AskQuestion tool** to confirm `$PROJECT_NAME` and to prompt for `$DOMAIN` if the user hasn't supplied one. An empty `$DOMAIN` is fine — the CLI omits the field. For hub mode, `$PROJECT_NAME` MUST be kebab-case (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens) — the CLI bakes it into `initiative.md`'s frontmatter and rejects non-kebab values.

   **Regular invocation:**

   ```bash
   specify init \
     --schema-uri "$SCHEMA_URI" \
     --name "$PROJECT_NAME" \
     ${DOMAIN:+--domain "$DOMAIN"}
   ```

   **Hub invocation** (when `$HUB_MODE=true`):

   ```bash
   specify init \
     --name "$PROJECT_NAME" \
     ${DOMAIN:+--domain "$DOMAIN"} \
     --hub
   ```

   The CLI writes:

   - **Regular** — `.specify/{changes,specs,archive,.cache}/`, `.specify/project.yaml` with one empty `rules:` entry per `pipeline.define` brief, the resolved schema cached under `.specify/.cache/`, `.specify/.cache/` upserted into `.gitignore`, and `specify-version` recorded.
   - **Hub** — `.specify/project.yaml` with `schema: hub`, `hub: true`, no `rules:` block; `registry.yaml` with `version: 1` and `projects: []`; `initiative.md` from the canonical template named after `$PROJECT_NAME`; `.specify/.cache/` and `.specify/workspace/` upserted into `.gitignore`. Phase-pipeline directories (`changes/`, `specs/`, `.cache/`) are NOT scaffolded — the hub disables those pipelines.

   For agent automation that needs structured output, add the global `--format json` flag before `init` and parse `config-path`, `schema-name`, `cache-present`, `directories-created`, `scaffolded-rule-keys`, `specify-version`, and `hub`. Normal operator-facing examples should use text output.

   On non-zero exit, surface the CLI error. Do not attempt a prose fallback. Hub mode in particular refuses to scaffold over an existing `.specify/` directory — if the user wants to convert an existing single-repo project into a hub, they remove `.specify/` first.

6. **Prompt for customization**

   For a **regular** init, tell the user:
   - "Specify initialized. Config written to `.specify/project.yaml`."
   - "Edit the `domain` field to describe your project's tech stack, architecture, and testing approach."
   - "Fill in the scaffolded `rules` entries to add project-level rules for specific artifacts. For fallback context, check the `domain` section in `.specify/.cache/<schema>/schema.yaml`."

   For a **hub** init, tell the user:
   - "Specify initialized as a registry-only platform hub. Config written to `.specify/project.yaml`."
   - "Add code projects to `registry.yaml` once they exist. The hub starts with `projects: []`."
   - "Edit `initiative.md` to frame the first initiative this hub will drive."

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

**Schema**: $SCHEMA_URI
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

**Schema**: $SCHEMA_URI
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
**Registry**: registry.yaml (`version: 1`, `projects: []`)
**Initiative brief**: initiative.md

Next steps:
1. Add registered projects to `registry.yaml` (hand-edit, or `specify registry add` once that verb lands)
2. Edit `initiative.md` to frame the first initiative
3. Run `/spec:plan <name>` to author a plan, then `/spec:execute --loop` to drive it
```

**Guardrails**
- `/spec:init` may install the CLI only after explicit user confirmation, using `cargo install --git https://github.com/augentic/specify-cli`
- Always verify `specify --version` before invoking `specify init`
- Do not overwrite an existing project.yaml without user confirmation
- For regular projects, pass a schema URI to `specify init --schema-uri`; do not hand-populate `.specify/.cache/`
- For hubs, never populate `.specify/.cache/` and never resolve a schema — the `hub` sentinel disables phase pipelines on the hub itself, so there is no schema to cache
- Hub init refuses to run over an existing `.specify/`; if the user wants to convert a regular project into a hub, they must remove `.specify/` first
- If the CLI exits non-zero, surface the error and stop; do not hand-roll the scaffold

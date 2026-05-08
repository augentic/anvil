---
name: specify-init
description: Initialize Specify in a project. Bootstraps the `specify` CLI when missing, decides between a regular single-project init and a registry-only platform hub, then invokes `specify init <capability>` (regular) or `specify init --hub` (hub) to scaffold `.specify/`, write `project.yaml`, and generate starter `AGENTS.md` context when absent. Use when setting up a new project for spec-driven development.
argument-hint: "<capability>"
---

## Critical Path (Quick Reference)

1. **Verify the CLI** — run `specify --version`; install with `cargo install --git https://github.com/augentic/specify-cli` only after explicit user confirmation.
2. **Check existing initialization** — detect `.specify/project.yaml`, ask before reinitializing, and treat reinit as an upgrade path owned by the CLI.
3. **Choose topology** — decide regular project vs registry-only platform hub; capability is required for regular projects and forbidden in hub mode.
4. **Resolve metadata** — choose `$CAPABILITY`, `$PROJECT_NAME`, and optional `$DOMAIN`; never pre-populate `.specify/.cache/`.
5. **Invoke `specify init`** — run either `specify init "$CAPABILITY" ...` or `specify init --hub ...`; let the CLI scaffold files and generate starter context, and surface non-zero CLI errors without hand-rolling scaffold files.
6. **Offer baseline extraction** — for regular projects with code indicators, ask whether to create `initial-baseline`; skip this entirely for hubs.
7. **Summarize the correct shape** — report regular vs hub outputs, next actions, and any baseline-extraction handoff.

## CLI bootstrap

`/spec:init` is the one Specify skill that may install the CLI before continuing. Other CLI-dependent skills still stop when `specify` is missing.

## Arguments

```text
$CAPABILITY     = $ARGUMENTS[0]
```

I'll ensure the `specify` CLI is available, decide whether this is a regular single-project init or a registry-only platform hub, then invoke `specify init <capability>` (regular) or `specify init --hub` (hub) to install a starter `project.yaml` and generated `AGENTS.md` context.

---

**Input**: None required. Optionally a capability identifier (a bare name like `omnia`, an `https://…` URL, or a `file:///…` URI) and project context. The capability argument is irrelevant for hub mode and must be omitted there.

**Capability vs `--hub` is mutually exclusive.** The CLI rejects both pathological invocations with the same diagnostic:

- `specify init` (no positional, no `--hub`) → exits with `init-requires-capability-or-hub`.
- `specify init <capability> --hub` (both supplied) → exits with `init-requires-capability-or-hub`.

A regular project must declare a capability; a hub must declare `--hub` and never carries a `capability:`. See [RFC-13 §Migration "Hub project shape"](../../../../rfcs/archive/rfc-13-extensibility.md#migration) for the post-cut-over shape.

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

   - **Regular project** — a single repository that contains both code and `.specify/`. The most common shape; choose this for single-repo projects, small teams, and any case where the operator just wants to track changes against the code in this repo. Phase pipelines (define / build / merge) run against this repo's working tree, driven by the active **capability**.
   - **Platform hub** (RFC-9 §1D) — a registry-only repository that holds platform state (`registry.yaml`, `change.md`, `plan.yaml`, `workspace/`) but never carries code itself. Choose this when the platform spans multiple repos and the operator wants the platform repo's identity to be unambiguous. Phase pipelines are disabled on the hub itself; code lives in registered project repos under `.specify/workspace/<name>/`.

   Ask the user via **AskQuestion tool** unless the answer is obvious from context (e.g. an existing `Cargo.toml` / `package.json` / `src/` strongly implies a regular project, while an empty directory in a multi-repo organisation often points at a hub). Treat the result as `$HUB_MODE=true|false`.

   Branch:

   - When `$HUB_MODE=true`, skip step 4's capability selection and jump to step 5's hub invocation.
   - When `$HUB_MODE=false`, continue with the capability-driven flow below.

4. **Choose capability** *(regular only — skip in hub mode)*

   If `$CAPABILITY` is provided (as an argument), use it directly. Otherwise, prefer the canonical Omnia capability identifier unless project context clearly indicates another capability:

   ```text
   https://github.com/augentic/specify/capabilities/omnia
   ```

   For local development in this repository, a local capability directory such as `./capabilities/omnia` is also valid. If multiple capabilities are plausible, use the **AskQuestion tool** to let the user select which one.

   Store the result as `$CAPABILITY`. Do not pre-populate `.specify/.cache/`; the CLI owns capability fetch/copy during `specify init <capability>`.

5. **Collect project metadata and invoke `specify init`**

   Determine `$PROJECT_NAME` (default: project directory basename) and optionally `$DOMAIN` (project description). Use the **AskQuestion tool** to confirm `$PROJECT_NAME` and to prompt for `$DOMAIN` if the user hasn't supplied one. An empty `$DOMAIN` is fine — the CLI omits the field. For hub mode, `$PROJECT_NAME` MUST be kebab-case (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens) — the CLI bakes it into `change.md`'s frontmatter and rejects non-kebab values.

   **Regular invocation** (capability is the required first positional):

   ```bash
   specify init "$CAPABILITY" \
     --name "$PROJECT_NAME" \
     ${DOMAIN:+--domain "$DOMAIN"}
   ```

   **Hub invocation** (when `$HUB_MODE=true` — no positional, `--hub` is the discriminator):

   ```bash
   specify init --hub \
     --name "$PROJECT_NAME" \
     ${DOMAIN:+--domain "$DOMAIN"}
   ```

   Never combine the two: `specify init "$CAPABILITY" --hub` errors with `init-requires-capability-or-hub`. `specify init` with neither supplied errors with the same diagnostic.

   The CLI writes:

   - **Regular** — `.specify/{slices,specs,archive,.cache}/`, `.specify/project.yaml` with `capability:` set to the resolved value and one empty `rules:` entry per `pipeline.define` brief, the resolved capability manifest cached under `.specify/.cache/`, `.specify/.cache/` upserted into `.gitignore`, `specify-version` recorded, and generated root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` was absent.
   - **Hub** — `.specify/project.yaml` with `hub: true` only (the `capability:` field is **omitted** — its absence is the sentinel that disables capability resolution on the hub itself), no `rules:` block; `registry.yaml` with `version: 1` and `projects: []`; `.specify/.cache/` and `.specify/workspace/` upserted into `.gitignore`; generated hub-shaped root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` was absent. Phase-pipeline directories (`slices/`, `specs/`, `.cache/`) are NOT scaffolded — the hub disables those pipelines. `change.md` and `plan.yaml` are minted later by their owning commands.

   If root `AGENTS.md` already exists, `specify init` preserves it byte-for-byte and prints `AGENTS.md already present; skipping context generate` in text mode. Init inside `.specify/workspace/<peer>/` also skips nested context generation.

   For agent automation that needs structured output, add the global `--format json` flag before `init` and parse `config-path`, `capability-name`, `cache-present`, `directories-created`, `scaffolded-rule-keys`, `specify-version`, `hub`, `context-generated`, `context-skipped`, and optional `context-skip-reason`. Normal operator-facing examples should use text output.

   On non-zero exit, surface the CLI error. Do not attempt a prose fallback. Hub mode in particular refuses to scaffold over an existing `.specify/` directory — if the user wants to convert an existing single-repo project into a hub, they remove `.specify/` first.

6. **Prompt for customization**

   For a **regular** init, tell the user:
   - "Specify initialized. Config written to `.specify/project.yaml`."
   - "Generated starter context at `AGENTS.md`; refresh it later with `specify context generate`."
   - "Edit the `domain` field to describe your project's tech stack, architecture, and testing approach."
   - "Fill in the scaffolded `rules` entries to add project-level rules for specific artifacts. For fallback context, check the `domain` section in `.specify/.cache/<capability>/capability.yaml`."

   For a **hub** init, tell the user:
   - "Specify initialized as a registry-only platform hub. Config written to `.specify/project.yaml` (`hub: true`, no `capability:`)."
   - "Generated hub context at `AGENTS.md`; refresh it later with `specify context generate`."
   - "Add code projects to `registry.yaml` once they exist. The hub starts with `projects: []`."

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
   - **Yes, generate baseline specs** — proceed to create the slice
   - **No, skip for now** — show the greenfield output and stop (user can run `/spec:extract` manually later)

   If the user chooses **yes**, create the slice via the CLI:

   ```bash
   specify slice create initial-baseline --format json
   ```

   The CLI validates the name, creates `.specify/slices/initial-baseline/specs/`, and writes the initial `.metadata.yaml` (status `defining`, `created_at` timestamp). Show the **brownfield output** and stop.

**Output (greenfield — regular project, no existing codebase, or user declined extraction)**

```
## Specify Initialized

**Capability**: $CAPABILITY
**Config**: .specify/project.yaml
**Context**: AGENTS.md
**Context lock**: .specify/context.lock
**Slices**: .specify/slices/
**Baseline specs**: .specify/specs/

Next steps:
1. Edit `.specify/project.yaml` to describe your project
2. Run `/spec:define` to create your first change
```

**Output (brownfield — regular project, user opted for baseline extraction)**

```
## Specify Initialized (Existing Codebase Detected)

**Capability**: $CAPABILITY
**Config**: .specify/project.yaml
**Context**: AGENTS.md
**Baseline change**: .specify/slices/initial-baseline/

Next steps:
1. Edit `.specify/project.yaml` to describe your project
2. Run `/spec:extract . .specify/slices/initial-baseline/` to analyze the codebase
3. After extraction, run `/spec:merge initial-baseline` to promote specs to baseline
4. Then run `/spec:define` for future changes
```

**Output (hub — `$HUB_MODE=true`)**

```
## Specify Initialized (Platform Hub)

**Topology**: registry-only hub (RFC-9 §1D)
**Config**: .specify/project.yaml (`hub: true`; `capability:` omitted)
**Context**: AGENTS.md
**Context lock**: .specify/context.lock
**Registry**: registry.yaml (`version: 1`, `projects: []`)

Next steps:
1. Add registered projects with `specify registry add`
2. Run `specify change create <name>` to frame the first change
3. Run `/change:plan <name>` to author a plan, then `/change:execute loop` to drive it
```

**Guardrails**
- `/spec:init` may install the CLI only after explicit user confirmation, using `cargo install --git https://github.com/augentic/specify-cli`
- Always verify `specify --version` before invoking `specify init`
- Do not overwrite an existing project.yaml without user confirmation
- For regular projects, pass the capability identifier (bare name or URL) as the **first positional argument** to `specify init`; do not hand-populate `.specify/.cache/`
- For hubs, never populate `.specify/.cache/` and never resolve a capability — the absence of `capability:` (paired with `hub: true`) disables phase pipelines on the hub itself, so there is nothing to cache
- Do not hand-roll `AGENTS.md` during init. The CLI generates it when absent, preserves an existing root `AGENTS.md`, and writes `.specify/context.lock` for `specify context check`.
- Never combine a capability positional with `--hub`; the CLI rejects that combination with `init-requires-capability-or-hub`
- Hub init refuses to run over an existing `.specify/`; if the user wants to convert a regular project into a hub, they must remove `.specify/` first
- If the CLI exits non-zero, surface the error and stop; do not hand-roll the scaffold

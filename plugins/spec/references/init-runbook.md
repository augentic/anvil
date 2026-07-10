# Init Skill Runbook

Operational detail for `/spec:init`. The SKILL.md keeps only the orientation surface (Critical Path + Reference table + Guardrails); everything procedural lives here.

## CLI bootstrap

`/spec:init` is the one Specify skill that may install the CLI before continuing. Other CLI-dependent skills still stop when `specify` is missing.

## Arguments

```text
$PROFILE     = $ARGUMENTS[0]
```

I'll ensure the `specify` CLI is available, decide whether this is a regular single-project init or a registry-only workspace, then invoke `specify init <adapter>` (regular) or `specify init --workspace` (workspace) to install a starter `project.yaml` and generated `AGENTS.md` context.

## Input

None required. Optionally a adapter identifier (a bare name like `omnia`, an `https://…` URL, or a `file:///…` URI) and project context. The adapter argument is irrelevant for workspace mode and must be omitted there.

**Adapter vs `--workspace` is mutually exclusive.** The CLI rejects both pathological invocations with clap's standard parse-error diagnostic and exit code `2`:

- `specify init` (no positional, no `--workspace`) → exits `2` with a missing-required-argument diagnostic.
- `specify init <adapter> --workspace` (both supplied) → exits `2` with an argument-conflict diagnostic.

A regular project must declare an adapter; a workspace must declare `--workspace` and never carries an `adapter:`.

## Steps

### 1. Ensure the CLI is available

Run:

```bash
specify --version
```

If the command succeeds, continue to step 2.

If `specify` is not on PATH, tell the user:

> "The `specify` CLI is required before I can initialize this project. I can install it now with `cargo install --git https://github.com/augentic/specify`, then verify `specify --version` before continuing."

Use the **AskQuestion tool** to confirm whether they want to install the CLI now.

- If they decline, stop and tell them to install the CLI manually, then re-run `/spec:init`.
- If they confirm, run:

  ```bash
  cargo install --git https://github.com/augentic/specify
  ```

After installation, run `specify --version` again.

- If verification succeeds, continue.
- If installation or verification fails, surface the error and stop. Do not attempt a prose fallback or hand-roll `.specify/` scaffolding.

These three probes run before any prompt the existing steps would otherwise fire. Each is a fast no-op when nothing has drifted, so prompt counts only grow when the operator must choose.

### 1b. Probe CLI version

Run:

```bash
specify upgrade --dry-run --format json
```

Parse the JSON body. The binary is **stale** when `to` differs from `from`, when `to` is `"HEAD"`, or when `head-fallback` is `true`. When `channel` is `"unknown"`, the CLI cannot self-update.

- If the binary is current (`to == from`), say nothing and continue to step 1c.
- If `channel` is `"unknown"`, surface the `guidance` string verbatim so the operator can upgrade manually, then continue to step 1c. Do not auto-run an upgrade.
- If stale on a known channel, tell the user:

  > "Your `specify` binary is behind the latest release (`<from>` → `<to>`). I can update it now with `specify upgrade --yes`."

  Use the **AskQuestion tool** to confirm.

  - If they decline, continue to step 1c on the current binary.
  - If they confirm, run:

    ```bash
    specify upgrade --yes
    ```

    Then print "CLI updated to `<to>`; no Cursor restart required." and continue to step 1c.

### 1c. Probe plugin cache

Run:

```bash
specify plugins doctor --format json
```

`doctor` never exits non-zero on drift — drift is a finding. Parse the JSON body: the cache is **drifted** when `summary.drifted > 0` or `summary.missing > 0`.

- If `summary.drifted` and `summary.missing` are both `0`, continue to step 2.
- If drifted, tell the user:

  > "Your Cursor plugin cache has drifted from the marketplace (`<drifted>` drifted, `<missing>` missing). I can clear it with `specify plugins refresh --yes`, but Cursor must restart to repopulate the cache."

  Use the **AskQuestion tool** to confirm.

  - If they decline, continue to step 2 on the current cache.
  - If they confirm, run:

    ```bash
    specify plugins refresh --yes
    ```

    The CLI prints `Plugin cache cleared. Restart Cursor to repopulate from the marketplace.` Relay that line, then **stop**: tell the operator to restart Cursor and re-run `/spec:init`. Do not continue to step 2 — the refreshed cache only repopulates on restart.

### 2. Check if already initialized

Check whether `.specify/project.yaml` exists.

- If it does not exist, this is a first-run init — continue to step 3.
- If it exists, inform the user: "Specify is already initialized in this project. Your config is at `.specify/project.yaml`."
- Use the **AskQuestion tool** to confirm whether they want to re-enter — a version upgrade that preserves `project.yaml` and every operator-authored artifact.
- If they decline, stop.
- If they confirm, elicit platforms when the existing target adapter requires them:

  1. Resolve the existing target adapter from `project.yaml` to check whether it declares `platforms.required`. When it does (e.g. vectis), use the **AskQuestion tool** to ask whether the operator wants to change the platform set:

     > "The current project targets `<current platforms from project.yaml>`. Do you want to change the platform set? The allowed set is `<allowed>` and `core` is mandatory."

     Options: **Keep current platforms** (recommended), **Change platforms (I'll specify)**.

     - If they choose to change, store the comma-separated result as `$PLATFORMS` (e.g. `core,ios,android`). Validate that `core` is present before proceeding.
     - If they keep current, leave `$PLATFORMS` unset.

  2. When the target does not declare `platforms.required`, skip the elicitation (`$PLATFORMS` is unset).

  3. Run the re-entry upgrade:

  ```bash
  specify init --upgrade ${PLATFORMS:+--platforms "$PLATFORMS"} --format json
  ```

  `--upgrade` bumps `specify-version`, preserves the existing `adapter:` (or `workspace:`) and all operator artifacts, and regenerates `AGENTS.md` only when absent. When `--platforms` is passed alongside `--upgrade`, the CLI resolves the existing target adapter, applies the same three validation rules (`project-platforms-required`, `project-platforms-must-include-core`, `project-platforms-not-allowed`), and updates the config's `platforms` field — this is the mutation affordance for changing platforms on an existing project (e.g. adding `android` to an iOS-only project). Branch on the JSON body's `specify-version-changed`:

  - `true` — the version was bumped. Report the new `specify-version` and `adapter-name` (or `"workspace"`); note that `AGENTS.md` was preserved when `context-skip-reason` is `"existing-agents-md"`. Stop — the project is already scaffolded.
  - `false` — already current; the run was an idempotent no-op. Tell the operator nothing changed and stop.

### 3. Decide the topology — regular project or workspace

See [Configuration files](https://specify.augentic.io/reference/configuration.html#projectyaml) and [Registry](https://specify.augentic.io/reference/registry.html) for the full background on the two shapes. Briefly:

- **Regular project** — a single repository that contains both code and `.specify/`. The most common shape; choose this for single-repo projects, small teams, and any case where the operator just wants to track changes against the code in this repo. Phase pipelines (define / build / merge) run against this repo's working tree, driven by the active **adapter**.
- **Workspace** — a registry-only repository that holds platform state (`registry.yaml`, `change.md`, `plan.yaml`, `workspace/`) but never carries code itself. Choose this when the platform spans multiple repos and the operator wants the platform repo's identity to be unambiguous. Phase pipelines are disabled on the workspace itself; code lives in registered project repos under `workspace/<name>/`.

Ask the user via **AskQuestion tool** unless the answer is obvious from context (e.g. an existing `Cargo.toml` / `package.json` / `src/` strongly implies a regular project, while an empty directory in a multi-repo organisation often points at a workspace). Treat the result as `$WORKSPACE_MODE=true|false`.

Branch:

- When `$WORKSPACE_MODE=true`, skip step 4's adapter selection and jump to step 5's workspace invocation.
- When `$WORKSPACE_MODE=false`, continue with the adapter-driven flow below.

### 4. Choose adapter *(regular only — skip in workspace mode)*

If `$PROFILE` is provided (as an argument), use it directly. Otherwise, prefer the canonical Omnia adapter identifier unless project context clearly indicates another adapter:

```text
https://github.com/augentic/specify/adapters/targets/omnia
```

For local development in this repository, a local target directory such as `./adapters/targets/omnia` is also valid. If multiple targets are plausible, use the **AskQuestion tool** to let the user select which one.

Store the result as `$PROFILE`. Do not pre-populate the out-of-tree per-project cache; the CLI owns adapter fetch/copy during `specify init <adapter>`.


### 4b. Elicit platforms *(regular only — skip in workspace mode)*

When the target adapter declares `platforms.required` (e.g. vectis), prompt the operator for the platform set before invoking `specify init`. The `default` and `allowed` sets come from the adapter's `metadata` answer (available from the `--format json` output of a dry-run resolve).

Use the **AskQuestion tool** to elicit the set:

> "The `<adapter>` target requires platform declarations. The default set is `<default>` (allowed: `<allowed>`). `core` is mandatory and must always be included. Which platforms should this project target?"

Options: present the `default` set as the recommended choice, with an "Other (I'll specify)" option for customisation.

Store the comma-separated result as `$PLATFORMS` (e.g. `core,ios,android`). When the operator picks a custom set, validate that `core` is present before proceeding — the CLI enforces this, but catching it early avoids a round-trip.

When the target does not declare `platforms.required`, skip this step entirely (`$PLATFORMS` is unset).

The upgrade path (step 2) has its own platform elicitation inline; this step applies only to first-run init.

### 5. Collect project metadata and invoke `specify init`

Determine `$PROJECT_NAME` (default: project directory basename) and optionally `$DESCRIPTION` (project description). Use the **AskQuestion tool** to confirm `$PROJECT_NAME` and to prompt for `$DESCRIPTION` if the user hasn't supplied one. An empty `$DESCRIPTION` is fine — the CLI omits the field. For workspace mode, `$PROJECT_NAME` MUST be kebab-case (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens) — the CLI bakes it into `change.md`'s frontmatter and rejects non-kebab values.

**Regular invocation** (adapter is the required first positional):

```bash
specify init "$PROFILE" \
  --name "$PROJECT_NAME" \
  ${PLATFORMS:+--platforms "$PLATFORMS"} \
  ${DESCRIPTION:+--description "$DESCRIPTION"}
```

**Workspace invocation** (when `$WORKSPACE_MODE=true` or `$PROFILE` is the literal `workspace` — no adapter positional, `--workspace` is the discriminator):

```bash
specify init --workspace \
  --name "$PROJECT_NAME" \
  ${DESCRIPTION:+--description "$DESCRIPTION"}
```

Never combine the two: `specify init "$PROFILE" --workspace` exits `2` with clap's argument-conflict diagnostic. `specify init` with neither supplied exits `2` with clap's missing-required-argument diagnostic.

The CLI writes:

- **Regular** — `.specify/{slices,specs,archive}/`, `.specify/project.yaml` with `adapter:` set to the resolved value; init scaffolds empty `rules:` entries for `proposal|specs|design|tasks`, the resolved adapter manifest cached in the out-of-tree per-project cache at `<project-cache>/manifests/targets/<adapter>/`, `.specify/scratch/` and top-level `workspace/` upserted into `.gitignore`, `specify-version` recorded, and generated root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` was absent.
- **Workspace** — `.specify/project.yaml` with `workspace: true` only (the `adapter:` field is **omitted** — its absence is the sentinel that disables adapter resolution on the workspace itself), no `rules:` block; `registry.yaml` with `version: 1` and `projects: []`; `.specify/scratch/` and top-level `workspace/` upserted into `.gitignore`; an initial `workspace sync` runs before init returns; generated workspace-shaped root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` was absent. Phase-pipeline directories (`slices/`, `specs/`) are NOT scaffolded — the workspace disables those pipelines. `change.md` and `plan.yaml` are minted later by their owning commands.

If root `AGENTS.md` already exists, `specify init` preserves it byte-for-byte and prints `AGENTS.md already present; skipping context generate` in text mode. Init inside `workspace/<peer>/` also skips nested context generation.

For agent automation that needs structured output, add the global `--format json` flag before `init` and parse `config-path`, `adapter-name`, `cache-present`, `directories-created`, `scaffolded-rule-keys`, `specify-version`, `workspace-synced`, `workspace-sync-message`, `context-generated`, `context-skipped`, and optional `context-skip-reason`. Normal operator-facing examples should use text output.

On non-zero exit, surface the CLI error. Do not attempt a prose fallback. Workspace mode in particular refuses to scaffold over an existing `.specify/` directory — if the user wants to convert an existing single-repo project into a workspace, they remove `.specify/` first.

### 6. Prompt for customization

For a **regular** init, tell the user:

- "Specify initialized. Config written to `.specify/project.yaml`."
- "Generated starter context at `AGENTS.md`; inspect the file directly for later review."
- "Edit the `description` field to describe your project's tech stack, architecture, and testing approach."
- "Fill in the scaffolded `rules` entries to add project-level rules for specific artifacts. For fallback context, check the target adapter's reference material in the adapters repo."

For a **workspace** init, tell the user:

- "Specify initialized as a registry-only workspace. Config written to `.specify/project.yaml` (`workspace: true`, no `adapter:`)."
- "Generated workspace context at `AGENTS.md`; inspect the file directly for later review."
- Report the init envelope's `workspace-sync-message` (CLI chains sync automatically — do not run `specify workspace sync` again).
- "Add code projects to `registry.yaml` once they exist. The workspace starts with `projects: []`."

Do NOT print "Next steps" yet — Step 7 determines which output to show.

### 7. Detect existing codebase and offer baseline extraction *(regular only — skip in workspace mode)*

When `$WORKSPACE_MODE=true`, skip this step entirely and show the **workspace output** below. A workspace never carries code, so codebase detection and baseline extraction do not apply.

For regular projects, check whether the project root contains an active codebase by looking for:

- **Manifest files**: `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `pom.xml`, `*.csproj`, `build.gradle`, `Gemfile`
- **Source directories**: `src/`, `lib/`, `app/`, `cmd/`

If **none** of these are found, show the **greenfield output** and stop.

If at least one indicator is found, use the **AskQuestion tool**:

> "I've detected an existing codebase (found `<indicator>`). Would you like me to analyze it and generate baseline specs that capture its current behavior? This is driven by `/spec:plan` with the matching language source adapter (e.g. `typescript`)."

Options:

- **Yes, generate baseline specs** — proceed to create the slice
- **No, skip for now** — show the greenfield output and stop (user can run `/spec:plan` manually later with the matching language source)

If the user chooses **yes**, create the slice via the CLI:

```bash
specify slice create initial-baseline --format json
```

The CLI validates the name, creates `.specify/slices/initial-baseline/specs/`, and writes the initial `metadata.yaml` (status `defining`, `created_at` timestamp). Show the **brownfield output** and stop.

## Output

Render the **greenfield** template for a regular project with no codebase indicators (or when the user declined extraction in step 7), the **brownfield** template after the user opted into baseline extraction, or the **workspace** template when `$WORKSPACE_MODE=true`. Each template substitutes the resolved `$PROFILE` (regular and brownfield only; workspace omits it). The verbatim templates live in [`init-output-templates.md`](init-output-templates.md).

## Skill scope

`/spec:init` keeps a narrow boundary; `plan.yaml` / `metadata.yaml` / archive moves are owned elsewhere per [shared guardrails](./guardrails.md#single-writer-for-lifecycle-state).

- **CLI-only scaffolding.** Never hand-roll `.specify/` when `specify init` fails — surface the error and stop. The CLI is the single writer for `.specify/`, `project.yaml`, root `AGENTS.md`, and `.specify/context.lock`.
- **No pre-cache.** Never pre-populate the out-of-tree per-project cache with adapter material — `specify init` owns adapter fetch and copy when invoked with the adapter positional.
- **Baseline extraction is delegated.** Init only creates the `initial-baseline` slice (via `specify slice create`) when the operator opts in; the actual extraction is driven by `/spec:plan` -> `specify plan execute`, with the bound language source adapter's `extract` prompt synthesizing evidence during `/spec:refine`.
- **No registry peer registration.** Workspace init only seeds an empty `projects: []`; peer registration lives in `specify registry add`.
- **Reinit is always confirmed.** Use the **AskQuestion tool** before treating the run as an upgrade.
- **Adapter vs `--workspace` is mutually exclusive.** The CLI rejects the combination with a clap parse error and exit code `2`; pick exactly one shape per run.

## References

- [Adapter anatomy](https://specify.augentic.io/explanation/adapter-anatomy.html) — adapter manifest boundaries and resolver behavior.
- [Lifecycle](https://specify.augentic.io/reference/lifecycle.html) — workflow state owned by CLI verbs.
- [`init-output-templates.md`](init-output-templates.md) — regular, brownfield, and workspace init summaries.

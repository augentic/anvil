# Init Skill Runbook

Operational detail for `/spec:init`. The SKILL.md keeps only the orientation surface (Critical Path + Reference table + Guardrails); everything procedural lives here.

## CLI bootstrap

`/spec:init` is the one Specify skill that may install the CLI before continuing. Other CLI-dependent skills still stop when `specrun` is missing.

## Arguments

```text
$PROFILE     = $ARGUMENTS[0]
```

I'll ensure the `specrun` CLI is available, decide whether this is a regular single-project init or a registry-only workspace root, then invoke `specrun init <adapter>` (regular) or `specrun init --workspace` (workspace root) to install a starter `project.yaml` and generated `AGENTS.md` context.

## Input

None required. Optionally a adapter identifier (a bare name like `omnia`, an `https://…` URL, or a `file:///…` URI) and project context. The adapter argument is irrelevant for workspace mode and must be omitted there.

**Adapter vs `--workspace` is mutually exclusive.** The CLI rejects both pathological invocations with clap's standard parse-error diagnostic and exit code `2`:

- `specrun init` (no positional, no `--workspace`) → exits `2` with a missing-required-argument diagnostic.
- `specrun init <adapter> --workspace` (both supplied) → exits `2` with an argument-conflict diagnostic.

A regular project must declare an adapter; a workspace root must declare `--workspace` and never carries an `adapter:`.

## Steps

### 1. Ensure the CLI is available

Run:

```bash
specrun --version
```

If the command succeeds, continue to step 2.

If `specrun` is not on PATH, tell the user:

> "The `specrun` CLI is required before I can initialize this project. I can install it now with `cargo install --git https://github.com/augentic/specify-cli`, then verify `specrun --version` before continuing."

Use the **AskQuestion tool** to confirm whether they want to install the CLI now.

- If they decline, stop and tell them to install the CLI manually, then re-run `/spec:init`.
- If they confirm, run:

  ```bash
  cargo install --git https://github.com/augentic/specify-cli
  ```

After installation, run `specrun --version` again.

- If verification succeeds, continue.
- If installation or verification fails, surface the error and stop. Do not attempt a prose fallback or hand-roll `.specify/` scaffolding.

These three probes run before any prompt the existing steps would otherwise fire. Each is a fast no-op when nothing has drifted, so prompt counts only grow when the operator must choose.

### 1b. Probe CLI version

Run:

```bash
specrun upgrade --dry-run --format json
```

Parse the JSON body. The binary is **stale** when `to` differs from `from`, when `to` is `"HEAD"`, or when `head-fallback` is `true`. When `channel` is `"unknown"`, the CLI cannot self-update.

- If the binary is current (`to == from`), say nothing and continue to step 1c.
- If `channel` is `"unknown"`, surface the `guidance` string verbatim so the operator can upgrade manually, then continue to step 1c. Do not auto-run an upgrade.
- If stale on a known channel, tell the user:

  > "Your `specrun` binary is behind the latest release (`<from>` → `<to>`). I can update it now with `specrun upgrade --yes`."

  Use the **AskQuestion tool** to confirm.

  - If they decline, continue to step 1c on the current binary.
  - If they confirm, run:

    ```bash
    specrun upgrade --yes
    ```

    Then print "CLI updated to `<to>`; no Cursor restart required." and continue to step 1c.

### 1c. Probe plugin cache

Run:

```bash
specrun plugins doctor --format json
```

`doctor` never exits non-zero on drift — drift is a finding. Parse the JSON body: the cache is **drifted** when `summary.drifted > 0` or `summary.missing > 0`.

- If `summary.drifted` and `summary.missing` are both `0`, continue to step 1d.
- If drifted, tell the user:

  > "Your Cursor plugin cache has drifted from the marketplace (`<drifted>` drifted, `<missing>` missing). I can clear it with `specrun plugins refresh --yes`, but Cursor must restart to repopulate the cache."

  Use the **AskQuestion tool** to confirm.

  - If they decline, continue to step 1d on the current cache.
  - If they confirm, run:

    ```bash
    specrun plugins refresh --yes
    ```

    The CLI prints `Plugin cache cleared. Restart Cursor to repopulate from the marketplace.` Relay that line, then **stop**: tell the operator to restart Cursor and re-run `/spec:init`. Do not continue to step 1d — the refreshed cache only repopulates on restart.

### 1d. Probe artifact major

Run:

```bash
specrun init --check-migration --format json
```

Parse the JSON body. Migration is required only when `needs-migration` is `true`. The CLI binary is pre-1.0 today, so the major-bump path cannot fire and `needs-migration` is virtually always `false` — treat that as the normal healthy result and continue to step 2.

- If `needs-migration` is `false`, continue to step 2.
- If `needs-migration` is `true`, the project's artifacts are pinned to an older major (`from`) than the binary targets (`to`). Tell the user:

  > "This project's artifacts are on Specify `<from>`; the CLI targets `<to>`. I can migrate them now with `specrun migrate --yes` before continuing."

  Use the **AskQuestion tool** to confirm.

  - If they decline, stop and tell them migration is required before any other Specify command can run.
  - If they confirm, run:

    ```bash
    specrun migrate --yes
    ```

    Render the **migrated** template (see [`init-output-templates.md`](init-output-templates.md)) from the migration report, then continue to step 2.

### 2. Check if already initialized

Check whether `.specify/project.yaml` exists.

- If it does not exist, this is a first-run init — continue to step 3.
- If it exists, inform the user: "Specify is already initialized in this project. Your config is at `.specify/project.yaml`."
- Use the **AskQuestion tool** to confirm whether they want to re-enter — a version upgrade that preserves `project.yaml` and every operator-authored artifact.
- If they decline, stop.
- If they confirm, run the re-entry upgrade:

  ```bash
  specrun init --upgrade --format json
  ```

  `--upgrade` bumps `specify-version`, preserves the existing `adapter:` (or `workspace:`) and all operator artifacts, and regenerates `AGENTS.md` only when absent. Branch on the JSON body's `specify-version-changed`:

  - `true` — the version was bumped. Report the new `specify-version` and `adapter-name` (or `"workspace"`); note that `AGENTS.md` was preserved when `context-skip-reason` is `"existing-agents-md"`. Stop — the project is already scaffolded.
  - `false` — already current; the run was an idempotent no-op. Tell the operator nothing changed and stop.

  If `specrun init --upgrade` exits `4` (`project-needs-migration`), run step 1d's migration handoff first, then retry the upgrade.

### 3. Decide the topology — regular project or workspace root

See [Configuration files](../../../docs/reference/configuration.md#projectyaml) and [Registry](../../../docs/reference/registry.md) for the full background on the two shapes. Briefly:

- **Regular project** — a single repository that contains both code and `.specify/`. The most common shape; choose this for single-repo projects, small teams, and any case where the operator just wants to track changes against the code in this repo. Phase pipelines (define / build / merge) run against this repo's working tree, driven by the active **adapter**.
- **Workspace root** — a registry-only repository that holds platform state (`registry.yaml`, `change.md`, `plan.yaml`, `workspace/`) but never carries code itself. Choose this when the platform spans multiple repos and the operator wants the platform repo's identity to be unambiguous. Phase pipelines are disabled on the workspace root itself; code lives in registered project repos under `.specify/workspace/<name>/`.

Ask the user via **AskQuestion tool** unless the answer is obvious from context (e.g. an existing `Cargo.toml` / `package.json` / `src/` strongly implies a regular project, while an empty directory in a multi-repo organisation often points at a workspace root). Treat the result as `$WORKSPACE_MODE=true|false`.

Branch:

- When `$WORKSPACE_MODE=true`, skip step 4's adapter selection and jump to step 5's workspace invocation.
- When `$WORKSPACE_MODE=false`, continue with the adapter-driven flow below.

### 4. Choose adapter *(regular only — skip in workspace mode)*

If `$PROFILE` is provided (as an argument), use it directly. Otherwise, prefer the canonical Omnia adapter identifier unless project context clearly indicates another adapter:

```text
https://github.com/augentic/specify/adapters/targets/omnia
```

For local development in this repository, a local target directory such as `./adapters/targets/omnia` is also valid. If multiple targets are plausible, use the **AskQuestion tool** to let the user select which one.

Store the result as `$PROFILE`. Do not pre-populate `.specify/.cache/`; the CLI owns adapter fetch/copy during `specrun init <adapter>`.

### 5. Collect project metadata and invoke `specrun init`

Determine `$PROJECT_NAME` (default: project directory basename) and optionally `$DESCRIPTION` (project description). Use the **AskQuestion tool** to confirm `$PROJECT_NAME` and to prompt for `$DESCRIPTION` if the user hasn't supplied one. An empty `$DESCRIPTION` is fine — the CLI omits the field. For workspace mode, `$PROJECT_NAME` MUST be kebab-case (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens) — the CLI bakes it into `change.md`'s frontmatter and rejects non-kebab values.

**Regular invocation** (adapter is the required first positional):

```bash
specrun init "$PROFILE" \
  --name "$PROJECT_NAME" \
  ${DESCRIPTION:+--description "$DESCRIPTION"}
```

**Workspace invocation** (when `$WORKSPACE_MODE=true` or `$PROFILE` is the literal `workspace` — no adapter positional, `--workspace` is the discriminator):

```bash
specrun init --workspace \
  --name "$PROJECT_NAME" \
  ${DESCRIPTION:+--description "$DESCRIPTION"}
```

Never combine the two: `specrun init "$PROFILE" --workspace` exits `2` with clap's argument-conflict diagnostic. `specrun init` with neither supplied exits `2` with clap's missing-required-argument diagnostic.

The CLI writes:

- **Regular** — `.specify/{slices,specs,archive,.cache}/`, `.specify/project.yaml` with `adapter:` set to the resolved value; init scaffolds empty `rules:` entries for `proposal|specs|design|tasks`, the resolved adapter manifest cached under `.specify/.cache/manifests/targets/<adapter>/`, `.specify/.cache/` upserted into `.gitignore`, `specify-version` recorded, and generated root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` was absent.
- **Workspace root** — `.specify/project.yaml` with `workspace: true` only (the `adapter:` field is **omitted** — its absence is the sentinel that disables adapter resolution on the workspace root itself), no `rules:` block; `registry.yaml` with `version: 1` and `projects: []`; `.specify/.cache/` and `.specify/workspace/` upserted into `.gitignore`; an initial `workspace sync` runs before init returns; generated workspace-shaped root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` was absent. Phase-pipeline directories (`slices/`, `specs/`, `.cache/`) are NOT scaffolded — the workspace root disables those pipelines. `change.md` and `plan.yaml` are minted later by their owning commands.

If root `AGENTS.md` already exists, `specrun init` preserves it byte-for-byte and prints `AGENTS.md already present; skipping context generate` in text mode. Init inside `.specify/workspace/<peer>/` also skips nested context generation.

For agent automation that needs structured output, add the global `--format json` flag before `init` and parse `config-path`, `adapter-name`, `cache-present`, `directories-created`, `scaffolded-rule-keys`, `specify-version`, `workspace-synced`, `workspace-sync-message`, `context-generated`, `context-skipped`, and optional `context-skip-reason`. Normal operator-facing examples should use text output.

On non-zero exit, surface the CLI error. Do not attempt a prose fallback. Workspace mode in particular refuses to scaffold over an existing `.specify/` directory — if the user wants to convert an existing single-repo project into a workspace root, they remove `.specify/` first.

### 6. Prompt for customization

For a **regular** init, tell the user:

- "Specify initialized. Config written to `.specify/project.yaml`."
- "Generated starter context at `AGENTS.md`; inspect the file directly for later review."
- "Edit the `description` field to describe your project's tech stack, architecture, and testing approach."
- "Fill in the scaffolded `rules` entries to add project-level rules for specific artifacts. For fallback context, check the `domain` section in `.specify/.cache/manifests/targets/<adapter>/adapter.yaml`."

For a **workspace** init, tell the user:

- "Specify initialized as a registry-only workspace root. Config written to `.specify/project.yaml` (`workspace: true`, no `adapter:`)."
- "Generated workspace context at `AGENTS.md`; inspect the file directly for later review."
- Report the init envelope's `workspace-sync-message` (CLI chains sync automatically — do not run `specrun workspace sync` again).
- "Add code projects to `registry.yaml` once they exist. The workspace root starts with `projects: []`."

Do NOT print "Next steps" yet — Step 7 determines which output to show.

### 7. Detect existing codebase and offer baseline extraction *(regular only — skip in workspace mode)*

When `$WORKSPACE_MODE=true`, skip this step entirely and show the **workspace output** below. A workspace root never carries code, so codebase detection and baseline extraction do not apply.

For regular projects, check whether the project root contains an active codebase by looking for:

- **Manifest files**: `Cargo.toml`, `package.json`, `go.mod`, `pyproject.toml`, `pom.xml`, `*.csproj`, `build.gradle`, `Gemfile`
- **Source directories**: `src/`, `lib/`, `app/`, `cmd/`

If **none** of these are found, show the **greenfield output** and stop.

If at least one indicator is found, use the **AskQuestion tool**:

> "I've detected an existing codebase (found `<indicator>`). Would you like me to analyze it and generate baseline specs that capture its current behavior? This is driven by `/spec:plan` with the matching `code-*` source adapter (e.g. `code-typescript`)."

Options:

- **Yes, generate baseline specs** — proceed to create the slice
- **No, skip for now** — show the greenfield output and stop (user can run `/spec:plan` manually later with the matching `code-*` source)

If the user chooses **yes**, create the slice via the CLI:

```bash
specrun slice create initial-baseline --format json
```

The CLI validates the name, creates `.specify/slices/initial-baseline/specs/`, and writes the initial `.metadata.yaml` (status `defining`, `created_at` timestamp). Show the **brownfield output** and stop.

## Output

Render the **greenfield** template for a regular project with no codebase indicators (or when the user declined extraction in step 7), the **brownfield** template after the user opted into baseline extraction, or the **workspace** template when `$WORKSPACE_MODE=true`. Each template substitutes the resolved `$PROFILE` (regular and brownfield only; workspace omits it). The verbatim templates live in [`init-output-templates.md`](init-output-templates.md).

## Skill scope

`/spec:init` keeps a narrow boundary; `plan.yaml` / `.metadata.yaml` / archive moves are owned elsewhere per [shared guardrails](../../../docs/standards/skill-guardrails.md#single-writer-for-lifecycle-state).

- **CLI-only scaffolding.** Never hand-roll `.specify/` when `specrun init` fails — surface the error and stop. The CLI is the single writer for `.specify/`, `project.yaml`, root `AGENTS.md`, and `.specify/context.lock`.
- **No pre-cache.** Never pre-populate `.specify/.cache/` with adapter material — `specrun init` owns adapter fetch and copy when invoked with the adapter positional.
- **Baseline extraction is delegated.** Init only creates the `initial-baseline` slice (via `specrun slice create`) when the operator opts in; the actual extraction is driven by `/spec:plan` -> `/spec:execute`, with the bound `code-*` source adapter's `extract` brief synthesizing evidence during `/spec:refine`.
- **No registry peer registration.** Workspace init only seeds an empty `projects: []`; peer registration lives in `specrun registry add`.
- **Reinit is always confirmed.** Use the **AskQuestion tool** before treating the run as an upgrade.
- **Adapter vs `--workspace` is mutually exclusive.** The CLI rejects the combination with a clap parse error and exit code `2`; pick exactly one shape per run.

## References

- [`docs/explanation/adapter-anatomy.md`](../../../docs/explanation/adapter-anatomy.md) — adapter manifest boundaries and resolver behavior.
- [`docs/reference/lifecycle.md`](../../../docs/reference/lifecycle.md) — workflow state owned by CLI verbs.
- [`init-output-templates.md`](init-output-templates.md) — regular, brownfield, and workspace init summaries.

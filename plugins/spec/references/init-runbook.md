# Init Skill Runbook

Operational detail for `/spec:init`. The SKILL.md keeps only the orientation surface (Critical Path + Reference table + Guardrails); everything procedural lives here.

## CLI bootstrap

`/spec:init` is the one Specify skill that may install the CLI before continuing. Other CLI-dependent skills still stop when `specify` is missing.

## Arguments

```text
$PROFILE     = $ARGUMENTS[0]
```

I'll ensure the `specify` CLI is available, decide whether this is a regular single-project init or a registry-only platform hub, then invoke `specify init <adapter>` (regular) or `specify init --hub` (hub) to install a starter `project.yaml` and generated `AGENTS.md` context.

## Input

None required. Optionally a adapter identifier (a bare name like `omnia`, an `https://…` URL, or a `file:///…` URI) and project context. The adapter argument is irrelevant for hub mode and must be omitted there.

**Adapter vs `--hub` is mutually exclusive.** The CLI rejects both pathological invocations with the same diagnostic:

- `specify init` (no positional, no `--hub`) → exits with `init-requires-adapter-or-hub`.
- `specify init <adapter> --hub` (both supplied) → exits with `init-requires-adapter-or-hub`.

A regular project must declare a adapter; a hub must declare `--hub` and never carries a `adapter:`.

## Steps

### 1. Ensure the CLI is available

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

### 2. Check if already initialized

Check whether `.specify/project.yaml` exists.

- If it exists, inform the user: "Specify is already initialized in this project. Your config is at `.specify/project.yaml`."
- Use **AskQuestion tool** to confirm whether they want to reinitialize (which overwrites project.yaml).
- If they decline, stop.
- If they confirm, treat the run as `$UPGRADE=true` so the CLI rewrites `specify-version` to the running binary.

### 3. Decide the topology — regular project or platform hub

See [Platform repo topologies](../../../docs/explanation/platform-repo.md) for the full background on the two shapes. Briefly:

- **Regular project** — a single repository that contains both code and `.specify/`. The most common shape; choose this for single-repo projects, small teams, and any case where the operator just wants to track changes against the code in this repo. Phase pipelines (define / build / merge) run against this repo's working tree, driven by the active **adapter**.
- **Platform hub** — a registry-only repository that holds platform state (`registry.yaml`, `change.md`, `plan.yaml`, `workspace/`) but never carries code itself. Choose this when the platform spans multiple repos and the operator wants the platform repo's identity to be unambiguous. Phase pipelines are disabled on the hub itself; code lives in registered project repos under `.specify/workspace/<name>/`.

Ask the user via **AskQuestion tool** unless the answer is obvious from context (e.g. an existing `Cargo.toml` / `package.json` / `src/` strongly implies a regular project, while an empty directory in a multi-repo organisation often points at a hub). Treat the result as `$HUB_MODE=true|false`.

Branch:

- When `$HUB_MODE=true`, skip step 4's adapter selection and jump to step 5's hub invocation.
- When `$HUB_MODE=false`, continue with the adapter-driven flow below.

### 4. Choose adapter *(regular only — skip in hub mode)*

If `$PROFILE` is provided (as an argument), use it directly. Otherwise, prefer the canonical Omnia adapter identifier unless project context clearly indicates another adapter:

```text
https://github.com/augentic/specify/adapters/omnia
```

For local development in this repository, a local adapter directory such as `./adapters/omnia` is also valid. If multiple adapters are plausible, use the **AskQuestion tool** to let the user select which one.

Store the result as `$PROFILE`. Do not pre-populate `.specify/.cache/`; the CLI owns adapter fetch/copy during `specify init <adapter>`.

### 5. Collect project metadata and invoke `specify init`

Determine `$PROJECT_NAME` (default: project directory basename) and optionally `$DOMAIN` (project description). Use the **AskQuestion tool** to confirm `$PROJECT_NAME` and to prompt for `$DOMAIN` if the user hasn't supplied one. An empty `$DOMAIN` is fine — the CLI omits the field. For hub mode, `$PROJECT_NAME` MUST be kebab-case (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens) — the CLI bakes it into `change.md`'s frontmatter and rejects non-kebab values.

**Regular invocation** (adapter is the required first positional):

```bash
specify init "$PROFILE" \
  --name "$PROJECT_NAME" \
  ${DOMAIN:+--domain "$DOMAIN"}
```

**Hub invocation** (when `$HUB_MODE=true` — no positional, `--hub` is the discriminator):

```bash
specify init --hub \
  --name "$PROJECT_NAME" \
  ${DOMAIN:+--domain "$DOMAIN"}
```

Never combine the two: `specify init "$PROFILE" --hub` errors with `init-requires-adapter-or-hub`. `specify init` with neither supplied errors with the same diagnostic.

The CLI writes:

- **Regular** — `.specify/{slices,specs,archive,.cache}/`, `.specify/project.yaml` with `adapter:` set to the resolved value and one empty `rules:` entry per `pipeline.define` brief, the resolved adapter manifest cached under `.specify/.cache/`, `.specify/.cache/` upserted into `.gitignore`, `specify-version` recorded, and generated root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` was absent.
- **Hub** — `.specify/project.yaml` with `hub: true` only (the `adapter:` field is **omitted** — its absence is the sentinel that disables adapter resolution on the hub itself), no `rules:` block; `registry.yaml` with `version: 1` and `projects: []`; `.specify/.cache/` and `.specify/workspace/` upserted into `.gitignore`; generated hub-shaped root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` was absent. Phase-pipeline directories (`slices/`, `specs/`, `.cache/`) are NOT scaffolded — the hub disables those pipelines. `change.md` and `plan.yaml` are minted later by their owning commands.

If root `AGENTS.md` already exists, `specify init` preserves it byte-for-byte and prints `AGENTS.md already present; skipping context generate` in text mode. Init inside `.specify/workspace/<peer>/` also skips nested context generation.

For agent automation that needs structured output, add the global `--format json` flag before `init` and parse `config-path`, `adapter-name`, `cache-present`, `directories-created`, `scaffolded-rule-keys`, `specify-version`, `hub`, `context-generated`, `context-skipped`, and optional `context-skip-reason`. Normal operator-facing examples should use text output.

On non-zero exit, surface the CLI error. Do not attempt a prose fallback. Hub mode in particular refuses to scaffold over an existing `.specify/` directory — if the user wants to convert an existing single-repo project into a hub, they remove `.specify/` first.

### 6. Prompt for customization

For a **regular** init, tell the user:

- "Specify initialized. Config written to `.specify/project.yaml`."
- "Generated starter context at `AGENTS.md`; refresh it later with `specify context generate`."
- "Edit the `domain` field to describe your project's tech stack, architecture, and testing approach."
- "Fill in the scaffolded `rules` entries to add project-level rules for specific artifacts. For fallback context, check the `domain` section in `.specify/.cache/<adapter>/adapter.yaml`."

For a **hub** init, tell the user:

- "Specify initialized as a registry-only platform hub. Config written to `.specify/project.yaml` (`hub: true`, no `adapter:`)."
- "Generated hub context at `AGENTS.md`; refresh it later with `specify context generate`."
- "Add code projects to `registry.yaml` once they exist. The hub starts with `projects: []`."

Do NOT print "Next steps" yet — Step 7 determines which output to show.

### 7. Detect existing codebase and offer baseline extraction *(regular only — skip in hub mode)*

When `$HUB_MODE=true`, skip this step entirely and show the **hub output** below. A hub never carries code, so codebase detection and baseline extraction do not apply.

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
specify slice create initial-baseline --format json
```

The CLI validates the name, creates `.specify/slices/initial-baseline/specs/`, and writes the initial `.metadata.yaml` (status `defining`, `created_at` timestamp). Show the **brownfield output** and stop.

## Output

Render the **greenfield** template for a regular project with no codebase indicators (or when the user declined extraction in step 7), the **brownfield** template after the user opted into baseline extraction, or the **hub** template when `$HUB_MODE=true`. Each template substitutes the resolved `$PROFILE` (regular and brownfield only; hub omits it). The verbatim templates live in [`init-output-templates.md`](init-output-templates.md).

## Skill scope

`/spec:init` keeps a narrow boundary; `plan.yaml` / `.metadata.yaml` / archive moves are owned elsewhere per [shared guardrails](../../references/guardrails.md#single-writer-for-lifecycle-state).

- **CLI-only scaffolding.** Never hand-roll `.specify/` when `specify init` fails — surface the error and stop. The CLI is the single writer for `.specify/`, `project.yaml`, root `AGENTS.md`, and `.specify/context.lock`.
- **No pre-cache.** Never pre-populate `.specify/.cache/` with adapter material — `specify init` owns adapter fetch and copy when invoked with the adapter positional.
- **Baseline extraction is delegated.** Init only creates the `initial-baseline` slice (via `specify slice create`) when the operator opts in; the actual extraction is driven by `/spec:plan` -> `/spec:execute`, with the bound `code-*` source adapter's `extract` brief synthesizing evidence during `/spec:refine`.
- **No registry peer registration.** Hub init only seeds an empty `projects: []`; peer registration lives in `specify registry add`.
- **Reinit is always confirmed.** Use the **AskQuestion tool** before treating the run as an upgrade.
- **Adapter vs `--hub` is mutually exclusive.** The CLI rejects the combination with `init-requires-adapter-or-hub`; pick exactly one shape per run.

## References

- [RFC-9: Platform](../../../rfcs/archive/rfc-9-platform.md) — registry-only platform hub topology.
- [RFC-13: Extensibility](../../../rfcs/archive/rfc-13-extensibility.md) — adapter vs `--hub` shape requirements.

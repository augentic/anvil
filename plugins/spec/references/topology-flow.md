# Topology flow — regular vs registry-only platform hub

`/spec:init` scaffolds one of two shapes. Pick exactly one; the CLI rejects the combinatorial cases.

## Decision tree

| Signal | Topology |
|---|---|
| Existing `Cargo.toml` / `package.json` / `pyproject.toml` / `src/` in the working directory | **Regular project** (most common) |
| Empty directory in a multi-repo organisation; the operator wants the platform repo's identity to be unambiguous | **Platform hub** (registry-only) |
| Anything ambiguous | Ask via the **AskQuestion tool**; set `$HUB_MODE=true|false` |

- **Regular project** — a single repository that contains both code and `.specify/`. Phase pipelines (define / build / merge) run against this repo's working tree, driven by the active **adapter**.
- **Platform hub** — a registry-only repository that holds platform state (`registry.yaml`, `change.md`, `plan.yaml`, `workspace/`) but never carries code. Phase pipelines are disabled on the hub itself; code lives in registered project repos under `.specify/workspace/<name>/`. See [Platform repo topologies](../../../docs/explanation/platform-repo.md) for the full background.

## Adapter vs `--hub` is mutually exclusive

The CLI rejects both pathological invocations with the same diagnostic:

- `specify init` (no positional, no `--hub`) → exits with `init-requires-adapter-or-hub`.
- `specify init <adapter> --hub` (both supplied) → exits with `init-requires-adapter-or-hub`.

A regular project must declare a adapter; a hub must declare `--hub` and never carries a `adapter:`.

## Metadata

Resolve before invoking the CLI:

- **`$PROFILE`** *(regular only — skip in workspace mode)* — if supplied as an argument, use it directly. Otherwise prefer the canonical Omnia target identifier (`https://github.com/augentic/specify/targets/omnia`) unless project context indicates another target. A local target directory (`./targets/omnia`) is also valid. If multiple candidates are plausible, use the **AskQuestion tool**. Do not pre-populate `.specify/.cache/`; the CLI owns target fetch/copy.
- **`$PROJECT_NAME`** — defaults to the project directory basename; confirm via the **AskQuestion tool**. For hub mode, `$PROJECT_NAME` MUST be kebab-case (lowercase ascii, digits, single hyphens; no leading/trailing/doubled hyphens) — the CLI bakes it into `change.md`'s frontmatter and rejects non-kebab values.
- **`$DOMAIN`** *(optional)* — project description. Empty is fine — the CLI omits the field.

## CLI invocation

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

Never combine the two — see [Adapter vs `--hub` is mutually exclusive](#adapter-vs---hub-is-mutually-exclusive).

For agent automation that needs structured output, add the global `--format json` flag before `init` and parse `config-path`, `adapter-name`, `cache-present`, `directories-created`, `scaffolded-rule-keys`, `specify-version`, `hub`, `context-generated`, `context-skipped`, and optional `context-skip-reason`. Normal operator-facing examples should use text output.

On non-zero exit, surface the CLI error. Do not attempt a prose fallback. Hub mode in particular refuses to scaffold over an existing `.specify/` directory — if the user wants to convert an existing single-repo project into a hub, they remove `.specify/` first.

## What the CLI writes

- **Regular** — `.specify/{slices,specs,archive,.cache}/`, `.specify/project.yaml` with `adapter:` set to the resolved value; init scaffolds empty `rules:` entries for `proposal|specs|design|tasks`, the resolved adapter manifest cached under `.specify/.cache/`, `.specify/.cache/` upserted into `.gitignore`, `specify-version` recorded, and generated root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` was absent.
- **Hub** — `.specify/project.yaml` with `hub: true` only (the `adapter:` field is **omitted** — its absence is the sentinel that disables adapter resolution on the hub itself), no `rules:` block; `registry.yaml` with `version: 1` and `projects: []`; `.specify/.cache/` and `.specify/workspace/` upserted into `.gitignore`; generated hub-shaped root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` was absent. Phase-pipeline directories (`slices/`, `specs/`, `.cache/`) are NOT scaffolded — the hub disables those pipelines. `change.md` and `plan.yaml` are minted later by their owning commands.

If root `AGENTS.md` already exists, `specify init` preserves it byte-for-byte and prints `AGENTS.md already present; skipping context generate` in text mode. Init inside `.specify/workspace/<peer>/` also skips nested context generation.

## Customization prompts

For a **regular** init, tell the user:

- "Specify initialized. Config written to `.specify/project.yaml`."
- "Generated starter context at `AGENTS.md`; refresh it later with `specify context generate`."
- "Edit the `domain` field to describe your project's tech stack, architecture, and testing approach."
- "Fill in the scaffolded `rules` entries to add project-level rules for specific artifacts. For fallback context, check the `domain` section in `.specify/.cache/<adapter>/adapter.yaml`."

For a **hub** init, tell the user:

- "Specify initialized as a registry-only platform hub. Config written to `.specify/project.yaml` (`hub: true`, no `adapter:`)."
- "Generated hub context at `AGENTS.md`; refresh it later with `specify context generate`."
- "Add code projects to `registry.yaml` once they exist. The hub starts with `projects: []`."

Do NOT print "Next steps" yet — [baseline-detection.md](baseline-detection.md) determines which output to show.

## Output dispatch

After step 6 of the Critical Path, render exactly one template from [init-output-templates.md](init-output-templates.md), substituting the resolved `$PROFILE` (regular and brownfield only; hub omits it):

| Resolution | Template |
|---|---|
| Regular project, no codebase indicators or user declined extraction | Greenfield |
| Regular project, user opted into baseline extraction | Brownfield |
| `$HUB_MODE=true` | Hub |

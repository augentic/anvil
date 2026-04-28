# specify init

Scaffold the `.specify/` project structure.

## Synopsis

```bash
specify init <schema> [--schema-dir <dir>] [--name <project-name>] [--domain "<description>"] [--upgrade] [--hub] [--format json]
```

## Description

Two modes, picked by the presence of `--hub`:

- **Regular** (no `--hub`): scaffolds a single-project workspace. Creates `.specify/{changes,specs,archive,.cache}/`, writes `.specify/project.yaml` with a `rules:` entry per `pipeline.define` brief, and reads the schema from `.specify/.cache/` (populated by the `/spec:init` skill before invoking the CLI). Records the running binary's version as `specify-version`.
- **Hub** (with `--hub`, RFC-9 §1D): scaffolds a registry-only platform hub. Creates `.specify/`, writes a sentinel `.specify/project.yaml { schema: hub, hub: true, … }`, an empty `.specify/registry.yaml { version: 1, projects: [] }`, and an `.specify/initiative.md` from the canonical template. The schema argument is ignored, no cache is needed, and phase-pipeline directories (`changes/`, `specs/`, `.cache/`) are NOT scaffolded — the hub disables those pipelines on itself. Refuses to run when `.specify/` already exists.

In both modes the command upserts `.specify/.cache/` and `.specify/workspace/` into the project `.gitignore`.

This is the CLI command invoked by [`/spec:init`](../../../plugins/spec/skills/init/SKILL.md). The skill adds interactive prompts (including the regular-vs-hub topology question), schema cache population, and project detection on top.

## Options

| Option | Description |
|--------|-------------|
| `schema` | Schema name or URL. Supports `@ref` suffix for version pinning. Ignored when `--hub` is set. |
| `--schema-dir` | Directory to resolve the schema from (defaults to `.specify/.cache/`). Ignored when `--hub` is set. |
| `--name` | Project name (defaults to the project directory basename). For hub mode, must be kebab-case (the CLI bakes it into `initiative.md`'s frontmatter). |
| `--domain` | Free-form domain description for the project |
| `--upgrade` | Re-run on an existing project to update `specify-version` to the running binary |
| `--hub` | Scaffold a registry-only platform hub instead of a regular project (RFC-9 §1D). Refuses to run when `.specify/` already exists. |
| `--format` | Output format: `json` for structured output |

## JSON output

When `--format json` is provided, returns:

- `config-path` -- path to the written `project.yaml`
- `schema-name` -- resolved schema name (or the literal string `hub` in hub mode)
- `cache-present` -- whether the schema cache was found (always `false` in hub mode)
- `directories-created` -- list of directories created
- `scaffolded-rule-keys` -- per-brief rule keys added to `project.yaml` (always empty in hub mode)
- `specify-version` -- version recorded in `project.yaml`
- `hub` -- `true` when `--hub` was passed, `false` otherwise

## See also

- [Platform repo topologies](../../explanation/platform-repo.md) -- when to choose hub vs platform-as-project
- [Configuration Files](../configuration.md) -- project.yaml and metadata format
- [Prerequisites](../../orientation/prerequisites.md) -- setup before first init
- [`specify registry`](registry.md) -- manage the hub's registry catalogue

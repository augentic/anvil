# specify init

Scaffold the `.specify/` project structure.

## Synopsis

```bash
specify init --schema-uri <uri> [--name <project-name>] [--domain "<description>"]
specify init --hub [--name <project-name>] [--domain "<description>"]
```

## Description

Two modes, picked by the presence of `--hub`:

- **Regular** (no `--hub`): scaffolds a single-project workspace. Creates `.specify/{changes,specs,archive,.cache}/`, resolves `--schema-uri` into `.specify/.cache/`, writes `.specify/project.yaml` with a `rules:` entry per `pipeline.define` brief, and records the running binary's version as `specify-version`.
- **Hub** (with `--hub`, RFC-9 §1D): scaffolds a registry-only platform hub. Creates `.specify/`, writes a sentinel `.specify/project.yaml { schema: hub, hub: true, … }`, an empty `registry.yaml { version: 1, projects: [] }`, and an `initiative.md` from the canonical template. No schema URI is needed, no cache is needed, and phase-pipeline directories (`changes/`, `specs/`, `.cache/`) are NOT scaffolded — the hub disables those pipelines on itself. Refuses to run when `.specify/` already exists.

In both modes the command upserts `.specify/.cache/` and `.specify/workspace/` into the project `.gitignore`.

This is the CLI command invoked by [`/spec:init`](../../../plugins/spec/skills/init/SKILL.md). The skill adds interactive prompts (including the regular-vs-hub topology question) and project detection on top.

## Options

| Option | Description |
|--------|-------------|
| `--schema-uri` | Schema URI to fetch or copy before scaffolding. Supports local schema directories, GitHub schema directory URLs, and `@ref` suffixes for version pinning. Required unless `--hub` is set. |
| `--name` | Project name (defaults to the project directory basename). For hub mode, must be kebab-case (the CLI bakes it into `initiative.md`'s frontmatter). |
| `--domain` | Free-form domain description for the project |
| `--hub` | Scaffold a registry-only platform hub instead of a regular project (RFC-9 §1D). Refuses to run when `.specify/` already exists. |
| `--format` | Global output format: `json` for structured automation output |

GitHub schema directory URIs are fetched via the local `git` executable, so existing Git credential helpers and configured Git auth are used.

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

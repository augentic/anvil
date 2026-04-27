# specify init

Scaffold the `.specify/` project structure.

## Synopsis

```bash
specify init <schema> [--schema-dir <dir>] [--name <project-name>] [--domain "<description>"] [--upgrade] [--format json]
```

## Description

Creates the `.specify/` directory with:

- `project.yaml` -- project configuration with schema reference, project name, and `specify-version` floor.
- `changes/` -- empty directory for active changes.
- `specs/` -- empty directory for baseline specs.
- `archive/` -- empty directory for finalised changes.
- `.cache/` entry added to `.gitignore`.

The CLI reads the schema from `.specify/.cache/` (populated by the `/spec:init` skill before invoking this command). It writes `project.yaml` with one empty `rules:` entry per `pipeline.define` brief, and records the running binary's version as `specify-version`.

This is the CLI command invoked by `/spec:init`. The skill adds interactive prompts, schema cache population, and project detection on top.

## Options

| Option | Description |
|--------|-------------|
| `schema` | Schema name or URL. Supports `@ref` suffix for version pinning. |
| `--schema-dir` | Directory to resolve the schema from (defaults to `.specify/.cache/`) |
| `--name` | Project name (defaults to the project directory basename) |
| `--domain` | Free-form domain description for the project |
| `--upgrade` | Re-run on an existing project to update `specify-version` to the running binary |
| `--format` | Output format: `json` for structured output |

## JSON output

When `--format json` is provided, returns:

- `config-path` -- path to the written `project.yaml`
- `schema-name` -- resolved schema name
- `cache-present` -- whether the schema cache was found
- `directories-created` -- list of directories created
- `scaffolded-rule-keys` -- per-brief rule keys added to `project.yaml`
- `specify-version` -- version recorded in `project.yaml`

## See also

- [/spec:init](../change-skills/init.md) -- skill that drives this command
- [Configuration Files](../configuration.md) -- project.yaml and metadata format
- [Prerequisites](../../orientation/prerequisites.md) -- setup before first init

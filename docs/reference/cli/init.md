# specify init

Scaffold the `.specify/` project structure.

## Synopsis

```bash
specify init [--schema <url>]
```

## Description

Creates the `.specify/` directory with:

- `project.yaml` -- project configuration with schema reference.
- `changes/` -- empty directory for active changes.
- `specs/` -- empty directory for baseline specs.
- `archive/` -- empty directory for finalised changes.

If `--schema` is provided, fetches and caches the schema files into `.specify/.cache/`.

This is the CLI command invoked by `/spec:init`. The skill adds interactive prompts and project detection on top.

## Options

| Option | Description |
|--------|-------------|
| `--schema <url>` | Schema URL to configure. Supports `@ref` suffix for version pinning. |

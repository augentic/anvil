# specify init

Scaffold the `.specify/` project structure and starter agent context.

## Synopsis

```bash
specify init <capability> [--name <project-name>] [--domain "<description>"]
specify init --hub [--name <project-name>] [--domain "<description>"]
```

## Description

Two modes, picked by the presence of `--hub`:

- **Regular** (positional `<capability>`): scaffolds a single-project workspace. Creates `.specify/{slices,specs,archive,.cache}/`, resolves the capability identifier into `.specify/.cache/`, writes `.specify/project.yaml` with `capability:` set and a `rules:` entry per `pipeline.define` brief, records the running binary's version as `specify-version`, and generates root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` is absent.
- **Hub** (with `--hub`, RFC-9 §1D): scaffolds a registry-only platform hub. Creates `.specify/`, writes a sentinel `.specify/project.yaml { hub: true, … }` (the `capability:` field is omitted — its absence is what disables capability resolution), creates an empty `registry.yaml { version: 1, projects: [] }` at the repo root, and generates hub-shaped root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` is absent. No capability identifier is needed, no cache is needed, and phase-pipeline directories (`slices/`, `specs/`, `.cache/`) are NOT scaffolded — the hub disables those pipelines on itself. `change.md` and `plan.yaml` are operator artifacts minted later by `specify change create` and `specify change plan create`. Refuses to run when `.specify/` already exists.

The two modes are mutually exclusive: `specify init` with neither a capability positional nor `--hub` errors with `init-requires-capability-or-hub`; passing both errors with the same diagnostic.

In both modes the command upserts `.specify/.cache/` and `.specify/workspace/` into the project `.gitignore`.

If root `AGENTS.md` already exists, `specify init` preserves it byte-for-byte and skips context generation. Init inside `.specify/workspace/<peer>/` also skips nested `AGENTS.md` generation; workspace clones inherit context from their owning project.

This is the CLI command invoked by [`/spec:init`](../../../plugins/spec/skills/init/SKILL.md). The skill adds interactive prompts (including the regular-vs-hub topology question) and project detection on top.

## Options

| Option | Description |
|--------|-------------|
| `<capability>` (positional) | Capability identifier or URL to fetch or copy before scaffolding. Accepts a bare name (e.g. `omnia`), an `https://…` capability directory URL, a `file:///…` URI, and `@ref` suffixes for version pinning. Required unless `--hub` is set. |
| `--name` | Project name (defaults to the project directory basename). For hub mode, must be kebab-case (the CLI bakes it into `change.md`'s frontmatter). |
| `--domain` | Free-form domain description for the project |
| `--hub` | Scaffold a registry-only platform hub instead of a regular project (RFC-9 §1D). Refuses to run when `.specify/` already exists. Mutually exclusive with the `<capability>` positional. |
| `--format` | Global output format: `json` for structured automation output |

GitHub capability directory URLs are fetched via the local `git` executable, so existing Git credential helpers and configured Git auth are used.

## JSON output

When `--format json` is provided, returns:

- `config-path` -- path to the written `project.yaml`
- `capability-name` -- resolved capability name (or the literal string `hub` in hub mode)
- `cache-present` -- whether the capability cache was found (always `false` in hub mode)
- `directories-created` -- list of directories created
- `scaffolded-rule-keys` -- per-brief rule keys added to `project.yaml` (always empty in hub mode)
- `specify-version` -- version recorded in `project.yaml`
- `hub` -- `true` when `--hub` was passed, `false` otherwise
- `context-generated` -- `true` when init generated root `AGENTS.md` and `.specify/context.lock`
- `context-skipped` -- `true` when context generation was skipped
- `context-skip-reason` -- present when skipped (`existing-agents-md` or `workspace-clone`)

## See also

- [Platform repo topologies](../../explanation/platform-repo.md) -- when to choose hub vs platform-as-project
- [Configuration Files](../configuration.md) -- project.yaml and metadata format
- [`specify context`](context.md) -- regenerate or check `AGENTS.md` context
- [Prerequisites](../../orientation/prerequisites.md) -- setup before first init
- [`specify registry`](registry.md) -- manage the hub's registry catalogue

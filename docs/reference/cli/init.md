# specify init

Scaffold the `.specify/` project structure and starter agent context.

## Synopsis

```bash
specify init <adapter> [--name <project-name>] [--domain "<description>"]
specify init --hub [--name <project-name>] [--domain "<description>"]
```

## Description

Two modes, picked by the presence of `--hub`:

- **Regular** (positional `<adapter>`): scaffolds a single-project workspace. Creates `.specify/{slices,specs,archive,.cache}/`, resolves the adapter identifier into `.specify/.cache/`, writes `.specify/project.yaml` with `adapter:` set and a `rules:` entry per `pipeline.define` brief, records the running binary's version as `specify-version`, and generates root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` is absent.
- **Hub** (with `--hub`): scaffolds a registry-only platform hub. Creates `.specify/`, writes a sentinel `.specify/project.yaml { hub: true, … }` (the `adapter:` field is omitted — its absence is what disables adapter resolution), creates an empty `registry.yaml { version: 1, projects: [] }` at the repo root, and generates hub-shaped root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` is absent. No adapter identifier is needed, no cache is needed, and phase-pipeline directories (`slices/`, `specs/`, `.cache/`) are NOT scaffolded — the hub disables those pipelines on itself. `change.md` and `plan.yaml` are operator artifacts minted later by `/spec:plan` (via `specify plan create`) (which scaffolds both files together). Refuses to run when `.specify/` already exists.

The two modes are mutually exclusive: `specify init` with neither a adapter positional nor `--hub` errors with `init-requires-adapter-or-hub`; passing both errors with the same diagnostic.

In both modes the command upserts `.specify/.cache/` and `.specify/workspace/` into the project `.gitignore`.

If root `AGENTS.md` already exists, `specify init` preserves it byte-for-byte and skips context generation. Init inside `.specify/workspace/<peer>/` also skips nested `AGENTS.md` generation; workspace clones inherit context from their owning project.

This is the CLI command invoked by [`/spec:init`](../../../plugins/spec/skills/init/SKILL.md). The skill adds interactive prompts (including the regular-vs-hub topology question) and project detection on top.

## Options

| Option | Description |
|--------|-------------|
| `<adapter>` (positional) | Adapter identifier or URL to fetch or copy before scaffolding. Accepts a bare name (e.g. `omnia`), an `https://…` adapter directory URL, a `file:///…` URI, and `@ref` suffixes for version pinning. Required unless `--hub` is set. |
| `--name` | Project name (defaults to the project directory basename). For hub mode, must be kebab-case (the CLI bakes it into `change.md`'s frontmatter). |
| `--domain` | Free-form domain description for the project |
| `--hub` | Scaffold a registry-only platform hub instead of a regular project. Refuses to run when `.specify/` already exists. Mutually exclusive with the `<adapter>` positional. |
| `--format` | Global output format: `json` for structured automation output |

GitHub adapter directory URLs are fetched via the local `git` executable, so existing Git credential helpers and configured Git auth are used.

## JSON output

When `--format json` is provided, returns:

- `config-path` -- path to the written `project.yaml`
- `adapter-name` -- resolved adapter name (or the literal string `hub` in hub mode)
- `cache-present` -- whether the adapter cache was found (always `false` in hub mode)
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

# specify init

Scaffold the `.specify/` project structure and starter agent context.

## Synopsis

```bash
specify init <adapter> [--name <project-name>] [--description "<description>"]
specify init --workspace [--name <project-name>] [--description "<description>"]
```

## Description

Two modes, picked by the presence of `--workspace`:

- **Regular** (positional `<adapter>`): scaffolds a single-project workspace. Creates `.specify/{slices,specs,archive}/`, resolves the adapter identifier into the out-of-tree per-project cache, writes `.specify/project.yaml` with `adapter:` set and a `rules:` entry per `pipeline.define` brief, records the running binary's version as `specify-version`, and generates root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` is absent.
- **Workspace** (with `--workspace`): scaffolds a registry-only workspace. Creates `.specify/`, writes a sentinel `.specify/project.yaml { workspace: true, … }` (the `adapter:` field is omitted — its absence is what disables adapter resolution), creates an empty `registry.yaml { version: 1, projects: [] }` at the repo root, and generates workspace-shaped root `AGENTS.md` plus `.specify/context.lock` when `AGENTS.md` is absent. No adapter identifier is needed, no cache is needed, and phase-pipeline directories (`slices/`, `specs/`) are NOT scaffolded — the workspace disables those pipelines on itself. Slot materialization is operator-owned. `change.md` and `plan.yaml` are operator artifacts minted later by `/spec:plan` (via `specify plan create`) (which scaffolds both files together). Refuses to run when `.specify/` already exists.

The two modes are mutually exclusive: `specify init` with both an adapter positional and `--workspace` exits `2` with clap's standard parse-error diagnostic. With neither, the elicitation layer engages: a line prompt for the adapter when stdin is a TTY, else the typed `init-adapter-required` error (exit `2`) naming the missing argument. When the resolved target requires `--platforms` and stdin is a TTY, init prompts for the platform set the same way; off a TTY the typed `project-platforms-required` names the flag and the default set.

Re-running `specify init` in an already-initialized project changes nothing and exits `0` with a message routing to `specify init --upgrade`.

On success init prints a postflight report: what was scaffolded, which pinned identities were hydrated (`<name>@<version>`), where the global adapter store lives, and the literal next command.

In both modes the command upserts `.specify/scratch/` and top-level `workspace/` into the project `.gitignore`.

If root `AGENTS.md` already exists, `specify init` preserves it byte-for-byte and skips context generation. Init inside `workspace/<peer>/` also skips nested `AGENTS.md` generation; workspace clones inherit context from their owning project.

This is the CLI command invoked by [`/spec:init`](../../../plugins/spec/skills/init/SKILL.md). The skill adds interactive prompts (including the regular-vs-workspace topology question) and project detection on top.

## Options

| Option | Description |
|--------|-------------|
| `<adapter>` (positional) | Adapter identifier or URL to fetch or copy before scaffolding. Accepts a bare name (e.g. `omnia`), an `https://…` adapter directory URL, a `file:///…` URI, and `@ref` suffixes for version pinning. Required unless `--workspace` is set. |
| `--name` | Project name (defaults to the project directory basename). For workspace mode, must be kebab-case (the CLI bakes it into `change.md`'s frontmatter). |
| `--description` | Free-form project description (tech stack, architecture, testing) |
| `--workspace` | Scaffold a registry-only workspace instead of a regular project. Refuses to run when `.specify/` already exists. Mutually exclusive with the `<adapter>` positional. |
| `--format` | Global output format: `json` for structured automation output |

GitHub adapter directory URLs are fetched via the local `git` executable, so existing Git credential helpers and configured Git auth are used.

## JSON output

When `--format json` is provided, returns:

- `config-path` -- path to the written `project.yaml`
- `adapter-name` -- resolved adapter name (or the literal string `workspace` in workspace mode)
- `cache-present` -- whether the adapter cache was found (always `false` in workspace mode)
- `directories-created` -- list of directories created
- `scaffolded-rule-keys` -- per-brief rule keys added to `project.yaml` (always empty in workspace mode)
- `specify-version` -- version recorded in `project.yaml`
- `hydrated` -- pinned identities hydrated into the global adapter store (`<name>@<version>`, bootstrap order; empty when components resolved locally)
- `adapter-store` -- root of the global adapter store
- `next` -- the literal next command for the operator
- `equivalent` -- the fully-flagged invocation, present only when a TTY prompt filled a missing argument
- `context-generated` -- `true` when init generated root `AGENTS.md` and `.specify/context.lock`
- `context-skipped` -- `true` when context generation was skipped
- `context-skip-reason` -- present when skipped (`existing-agents-md` or `workspace-clone`)


## See also

- [Configuration files](../../reference/configuration.md#projectyaml) and [Registry](../../reference/registry.md) -- when to choose a workspace vs platform-as-project
- [Configuration Files](../configuration.md) -- project.yaml and metadata format
- `AGENTS.md` context is generated during `specify init`; later inspection is direct file review.
- [Prerequisites](../../orientation/prerequisites.md) -- setup before first init
- [`specify registry`](registry.md) -- manage the workspace's registry catalogue
